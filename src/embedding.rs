use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::api::sync::Api;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

const MODEL_NAME: &str = "intfloat/multilingual-e5-small";
const EMBEDDING_DIMENSION: usize = 384;

lazy_static! {
    static ref EMBEDDING_SERVICE: Arc<Mutex<Option<EmbeddingService>>> = Arc::new(Mutex::new(None));
}

pub struct EmbeddingService {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl EmbeddingService {
    /// 埋め込みサービスを初期化
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("[EmbeddingService] モデルをロード中: {}...", MODEL_NAME);
        let start_time = std::time::Instant::now();

        let device = Device::Cpu;
        
        // Hugging Face Hubからモデルをダウンロード
        let api = Api::new()?;
        let repo = api.model(MODEL_NAME.to_string());
        
        println!("[EmbeddingService] config.jsonをダウンロード中...");
        let config_filename = repo.get("config.json")?;
        println!("[EmbeddingService] tokenizer.jsonをダウンロード中...");
        let tokenizer_filename = repo.get("tokenizer.json")?;
        println!("[EmbeddingService] model.safetensorsをダウンロード中...");
        let weights_filename = repo.get("model.safetensors")?;

        // Config読み込み
        let config = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config)?;

        // Tokenizer読み込み
        let tokenizer = Tokenizer::from_file(tokenizer_filename)
            .map_err(|e| format!("Tokenizer読み込みエラー: {}", e))?;

        // モデルウェイトの読み込み
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], candle_core::DType::F32, &device)? };
        let model = BertModel::load(vb, &config)?;

        let load_time = start_time.elapsed();
        println!("[EmbeddingService] モデルロード完了 ({:.2}秒)", load_time.as_secs_f64());
        println!("[EmbeddingService] 埋め込み次元数: {}", EMBEDDING_DIMENSION);

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// グローバルインスタンスを初期化
    pub fn initialize_global() -> Result<(), Box<dyn std::error::Error>> {
        let mut service = EMBEDDING_SERVICE.lock().unwrap();
        if service.is_none() {
            *service = Some(Self::new()?);
        }
        Ok(())
    }

    /// グローバルインスタンスを取得
    #[allow(dead_code)]
    pub fn get_global() -> Arc<Mutex<Option<EmbeddingService>>> {
        EMBEDDING_SERVICE.clone()
    }

    /// テキストをベクトル化
    pub fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        if text.trim().is_empty() {
            return Err("テキストが空です".into());
        }

        // トークン化
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| format!("トークン化エラー: {}", e))?;
        
        let tokens = encoding.get_ids();
        
        // トークン数を512に制限（BERTの最大長）
        const MAX_TOKENS: usize = 510; // [CLS]と[SEP]用に2トークン予約
        
        if tokens.len() <= MAX_TOKENS {
            // 通常処理
            return self.encode_tokens(tokens);
        }
        
        // 長いテキストの場合: チャンク分割して平均を取る
        println!(
            "[EmbeddingService] 長いテキスト検出: {}トークン → チャンク分割して処理",
            tokens.len()
        );
        
        let mut chunk_embeddings = Vec::new();
        let stride = MAX_TOKENS / 2; // 50%オーバーラップ
        
        for start in (0..tokens.len()).step_by(stride) {
            let end = (start + MAX_TOKENS).min(tokens.len());
            let chunk = &tokens[start..end];
            
            if chunk.is_empty() {
                continue;
            }
            
            match self.encode_tokens(chunk) {
                Ok(emb) => chunk_embeddings.push(emb),
                Err(e) => {
                    eprintln!("[EmbeddingService] チャンクのベクトル化エラー: {}", e);
                    continue;
                }
            }
            
            // 最後のチャンクまで到達したら終了
            if end >= tokens.len() {
                break;
            }
        }
        
        if chunk_embeddings.is_empty() {
            return Err("チャンク分割後にベクトル化できませんでした".into());
        }
        
        // 全チャンクの平均を計算
        println!("[EmbeddingService] {}個のチャンクの平均を計算", chunk_embeddings.len());
        let mut avg_embedding = vec![0.0f32; EMBEDDING_DIMENSION];
        
        for emb in &chunk_embeddings {
            for (i, &val) in emb.iter().enumerate() {
                avg_embedding[i] += val;
            }
        }
        
        let n = chunk_embeddings.len() as f32;
        for val in &mut avg_embedding {
            *val /= n;
        }
        
        // L2正規化
        let norm: f32 = avg_embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut avg_embedding {
                *val /= norm;
            }
        }
        
        Ok(avg_embedding)
    }
    
    /// トークン列をベクトル化（内部ヘルパー関数）
    fn encode_tokens(&self, tokens: &[u32]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let token_ids = Tensor::new(tokens, &self.device)?
            .unsqueeze(0)?; // バッチ次元追加

        // モデル実行
        let token_type_ids = token_ids.zeros_like()?;
        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;
        
        // Mean pooling
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings_sum = embeddings.sum(1)?;
        let embeddings_avg = (embeddings_sum / (n_tokens as f64))?;

        // L2正規化
        let embeddings_norm = embeddings_avg.broadcast_div(
            &embeddings_avg.sqr()?.sum_keepdim(1)?.sqrt()?
        )?;

        // テンソルからVec<f32>に変換
        let embedding = embeddings_norm
            .squeeze(0)?
            .to_vec1::<f32>()?;

        // 次元数の検証
        if embedding.len() != EMBEDDING_DIMENSION {
            eprintln!(
                "[EmbeddingService] 警告: 予期しない埋め込み次元数: {} (期待値: {})",
                embedding.len(),
                EMBEDDING_DIMENSION
            );
        }

        Ok(embedding)
    }

    /// コサイン類似度を計算
    pub fn cosine_similarity(vec_a: &[f32], vec_b: &[f32]) -> Result<f32, Box<dyn std::error::Error>> {
        if vec_a.len() != vec_b.len() {
            return Err(format!(
                "ベクトル次元の不一致: {} vs {}",
                vec_a.len(),
                vec_b.len()
            ).into());
        }

        if vec_a.is_empty() {
            return Err("ベクトルが空です".into());
        }

        // 内積計算
        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..vec_a.len() {
            dot_product += vec_a[i] * vec_b[i];
            norm_a += vec_a[i] * vec_a[i];
            norm_b += vec_b[i] * vec_b[i];
        }

        // ノルムが0の場合（ゼロベクトル）の処理
        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        // コサイン類似度 = 内積 / (ノルムA * ノルムB)
        let similarity = dot_product / (norm_a.sqrt() * norm_b.sqrt());

        Ok(similarity)
    }

    /// モデル情報を取得
    #[allow(dead_code)]
    pub fn get_model_info(&self) -> ModelInfo {
        ModelInfo {
            model: MODEL_NAME.to_string(),
            dimension: EMBEDDING_DIMENSION,
        }
    }
}

#[allow(dead_code)]
pub struct ModelInfo {
    pub model: String,
    pub dimension: usize,
}

/// ヘルパー関数: グローバルサービスでテキストをベクトル化
pub fn generate_embedding_global(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let service = EMBEDDING_SERVICE.lock().unwrap();
    match service.as_ref() {
        Some(s) => s.generate_embedding(text),
        None => Err("EmbeddingServiceが初期化されていません".into()),
    }
}

/// ヘルパー関数: コサイン類似度計算
pub fn cosine_similarity(vec_a: &[f32], vec_b: &[f32]) -> Result<f32, Box<dyn std::error::Error>> {
    EmbeddingService::cosine_similarity(vec_a, vec_b)
}

