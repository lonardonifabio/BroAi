use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tracing::{error, info, instrument, warn};

use crate::errors::AppError;

const QUEUE_CAPACITY: usize = 32;
const DEFAULT_INFERENCE_TIMEOUT_SECS: u64 = 300;
const N_CTX: u32 = 2048;
const DEFAULT_N_THREADS: u32 = 4;
const MAX_GENERATION_TOKENS: u32 = 512;

#[allow(dead_code)]
struct InferRequest {
    prompt: String,
    max_tokens: u32,
    temperature: f32,
    reply: oneshot::Sender<Result<String, AppError>>,
}

#[derive(Clone)]
pub struct LlmActor {
    sender: mpsc::Sender<InferRequest>,
    model_name: Arc<String>,
    ready: Arc<std::sync::atomic::AtomicBool>,
}

impl LlmActor {
    pub fn spawn(model_path: String) -> Result<Self, AppError> {
        let (tx, rx) = mpsc::channel::<InferRequest>(QUEUE_CAPACITY);
        let model_name = Arc::new(
            std::path::Path::new(&model_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown-model")
                .to_string(),
        );
        let ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ready_clone = ready.clone();

        std::thread::spawn(move || {
            worker_loop(model_path, rx, ready_clone);
        });

        Ok(Self {
            sender: tx,
            model_name,
            ready,
        })
    }

    #[instrument(skip(self, prompt))]
    pub async fn infer(
        &self,
        prompt: String,
        max_tokens: u32,
        temperature: f32,
    ) -> Result<String, AppError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender
            .try_send(InferRequest {
                prompt,
                max_tokens,
                temperature,
                reply: reply_tx,
            })
            .map_err(|_| AppError::QueueFull)?;

        let timeout_secs = inference_timeout_secs();
        timeout(Duration::from_secs(timeout_secs), reply_rx)
            .await
            .map_err(|_| AppError::Timeout(timeout_secs))?
            .map_err(|_| AppError::Cancelled)?
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn model_name(&self) -> String {
        (*self.model_name).clone()
    }
}

fn worker_loop(
    model_path: String,
    mut rx: mpsc::Receiver<InferRequest>,
    ready: Arc<std::sync::atomic::AtomicBool>,
) {
    info!(model_path = %model_path, "LLM worker starting");

    if !std::path::Path::new(&model_path).exists() {
        warn!(
            "Model not found at '{}' — running in MOCK mode.",
            model_path
        );
        ready.store(true, std::sync::atomic::Ordering::Relaxed);
        info!("LLM worker ready (mock mode)");
        while let Some(req) = rx.blocking_recv() {
            if req.reply.send(mock_infer(&req.prompt)).is_err() {
                warn!("Client disconnected before response was delivered");
            }
        }
        return;
    }

    // --- Real llama.cpp inference ---
    use llama_cpp::{LlamaModel, LlamaParams};

    info!("Loading model from disk, please wait...");

    let model = match LlamaModel::load_from_file(&model_path, LlamaParams::default()) {
        Ok(m) => {
            info!("Model loaded successfully");
            m
        }
        Err(e) => {
            error!(error = %e, "Failed to load model");
            ready.store(true, std::sync::atomic::Ordering::Relaxed);
            while let Some(req) = rx.blocking_recv() {
                let _ = req
                    .reply
                    .send(Err(AppError::LlmError(format!("Model load failed: {}", e))));
            }
            return;
        }
    };

    ready.store(true, std::sync::atomic::Ordering::Relaxed);
    info!("LLM worker ready (real inference mode)");

    while let Some(req) = rx.blocking_recv() {
        let result = real_infer(&model, &req.prompt, req.max_tokens, req.temperature);
        if req.reply.send(result).is_err() {
            warn!("Client disconnected before response was delivered");
        }
    }

    info!("LLM worker shutting down");
}

fn real_infer(
    model: &llama_cpp::LlamaModel,
    prompt: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<String, AppError> {
    use llama_cpp::standard_sampler::{SamplerStage, StandardSampler};
    use llama_cpp::SessionParams;

    let n_threads = inference_threads();
    let mut ctx = model
        .create_session(SessionParams {
            n_ctx: N_CTX,
            n_threads,
            n_threads_batch: n_threads,
            ..Default::default()
        })
        .map_err(|e| AppError::LlmError(format!("Failed to create session: {}", e)))?;

    ctx.advance_context(prompt)
        .map_err(|e| AppError::LlmError(format!("Failed to advance context: {}", e)))?;

    let requested_tokens = max_tokens.clamp(1, MAX_GENERATION_TOKENS) as usize;
    let normalized_temperature = temperature.clamp(0.0, 2.0);

    let sampler = StandardSampler::new_softmax(
        vec![
            SamplerStage::RepetitionPenalty {
                repetition_penalty: 1.1,
                frequency_penalty: 0.0,
                presence_penalty: 0.0,
                last_n: 64,
            },
            SamplerStage::TopK(40),
            SamplerStage::TopP(0.95),
            SamplerStage::MinP(0.05),
            SamplerStage::Temperature(normalized_temperature),
        ],
        1,
    );

    let completions = ctx
        .start_completing_with(sampler, requested_tokens)
        .map_err(|e| AppError::LlmError(format!("Failed to start completion: {}", e)))?
        .into_strings();

    // Apply a second .take() guard in case a downstream iterator ignores token bounds.
    let output: String = completions.take(requested_tokens).collect();

    Ok(output.trim().to_string())
}

fn inference_timeout_secs() -> u64 {
    std::env::var("INFERENCE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_INFERENCE_TIMEOUT_SECS)
}

fn inference_threads() -> u32 {
    let auto_threads = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(DEFAULT_N_THREADS);

    std::env::var("LLM_THREADS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(auto_threads)
}

fn mock_infer(prompt: &str) -> Result<String, AppError> {
    let words = prompt.split_whitespace().count();
    Ok(format!(
        "[MOCK] Prompt had {} words. Set MODEL_PATH to a valid .gguf file for real inference.",
        words
    ))
}
