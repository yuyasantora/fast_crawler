use anyhow::{anyhow, Result};
use candle_core::{quantized::gguf_file, Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_qwen2 as model;
use hf_hub::{api::tokio::Api, Repo, RepoType};
use std::io::Write;
use tokenizers::Tokenizer; // print!で逐次出力するため

pub struct LlmEngine {
    model: model::ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
}

impl LlmEngine {
    pub async fn new() -> Result<Self> {
        println!("⏳ Initializing Candle Engine...");

        // 1. CUDAチェック
        let device = Device::new_cuda(0).unwrap_or_else(|e| {
            println!("⚠️ CUDA Error: {}. Falling back to CPU.", e);
            Device::Cpu
        });
        println!("🚀 Device: {:?}", device);

        // 2. モデルのダウンロード
        let api = Api::new()?;

        // tokenizer は通常版リポジトリから取得（GGUFには含まれない）
        let tokenizer_repo = api.repo(Repo::new(
            "Qwen/Qwen2.5-14B-Instruct".to_string(),
            RepoType::Model,
        ));
        println!("📥 Downloading/Loading tokenizer...");
        let tokenizer_path = tokenizer_repo.get("tokenizer.json").await?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| anyhow!(e))?;

        // GGUF weights は bartowski リポジトリから取得（単一ファイル版）
        let gguf_repo = api.repo(Repo::new(
            "bartowski/Qwen2.5-14B-Instruct-GGUF".to_string(),
            RepoType::Model,
        ));
        println!("📥 Downloading/Loading model weights (Q4_K_M, ~10GB)...");
        let model_path = gguf_repo.get("Qwen2.5-14B-Instruct-Q4_K_M.gguf").await?;

        let mut file = std::fs::File::open(&model_path)?;
        let gguf_content = gguf_file::Content::read(&mut file)?;

        // メモリへのロード
        let model = model::ModelWeights::from_gguf(gguf_content, &mut file, &device)?;

        println!("✅ Engine Ready.");
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    pub fn generate(&mut self, system: &str, user: &str) -> Result<String> {
        use std::time::Instant;

        // Qwen特有のChatMLフォーマット
        let prompt_str = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            system, user
        );

        // トークナイズ
        let tokens = self
            .tokenizer
            .encode(prompt_str, true)
            .map_err(|e| anyhow!(e))?;
        let prompt_tokens = tokens.get_ids().to_vec();
        let input_tokens_count = prompt_tokens.len();

        let mut generated_tokens = Vec::new();

        // 生成パラメータ
        let mut logits_processor = LogitsProcessor::new(1337, Some(0.2), Some(0.95));
        let eos_token = *self
            .tokenizer
            .get_vocab(true)
            .get("<|im_end|>")
            .unwrap_or(&151645);

        // === 計測開始: プロンプト処理 ===
        let prefill_start = Instant::now();

        // 1. プロンプト全体を forward して KV cache を構築
        let input = Tensor::new(prompt_tokens.as_slice(), &self.device)?.unsqueeze(0)?;
        let logits = self.model.forward(&input, 0)?;
        let logits = logits.squeeze(0)?; // [vocab_size]

        let prefill_time = prefill_start.elapsed();

        // 最初のトークンをサンプリング
        let mut next_token = logits_processor.sample(&logits)?;
        let mut pos = prompt_tokens.len();

        // === 計測開始: トークン生成 ===
        let gen_start = Instant::now();

        // 2. 生成ループ
        for _ in 0..1000 {
            generated_tokens.push(next_token);

            // プログレス表示（100トークンごと）
            if generated_tokens.len() % 100 == 0 {
                print!(".");
                std::io::stdout().flush()?;
            }

            if next_token == eos_token {
                break;
            }

            // Forward Pass (1トークンずつ)
            let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?;

            // Sampling
            next_token = logits_processor.sample(&logits)?;
            pos += 1;
        }

        let gen_time = gen_start.elapsed();
        let total_time = prefill_time + gen_time;
        let gen_tokens_count = generated_tokens.len();
        let tokens_per_sec = gen_tokens_count as f64 / gen_time.as_secs_f64();

        // === 計測結果を出力 ===
        println!("\n📊 LLM Stats:");
        println!("   Input:    {} tokens ({:.2}s prefill)", input_tokens_count, prefill_time.as_secs_f64());
        println!("   Output:   {} tokens ({:.2}s generation)", gen_tokens_count, gen_time.as_secs_f64());
        println!("   Speed:    {:.1} tokens/sec", tokens_per_sec);
        println!("   Total:    {:.2}s", total_time.as_secs_f64());

        // 最終的な文字列を返す
        let result = self
            .tokenizer
            .decode(&generated_tokens, true)
            .map_err(|e| anyhow!(e))?;
        Ok(result)
    }
}
