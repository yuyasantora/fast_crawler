mod llm;
mod plugins;
mod summarizer;
mod traits;

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use llm::LlmEngine;
use axum::extract::Path;
use axum::response::Response;
use plugins::ip_force::{search_judgments, IpForcePatent, SearchResult};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use traits::WebResource;

// 共有状態
struct AppState {
    llm: Mutex<LlmEngine>,
}

// リクエスト/レスポンス
#[derive(Deserialize)]
struct AnalyzeRequest {
    case_id: u32,
}

#[derive(Serialize)]
struct AnalyzeResponse {
    success: bool,
    title: Option<String>,
    case_no: Option<String>,
    pdf_path: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

// 検索リクエスト/レスポンス
#[derive(Deserialize)]
struct SearchRequest {
    keyword: Option<String>,
    kenri: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct SearchResponse {
    success: bool,
    results: Vec<SearchResult>,
    error: Option<String>,
}

// ヘルスチェック
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: "0.1.0".to_string(),
    })
}

// 検索 API
async fn search(Json(req): Json<SearchRequest>) -> (StatusCode, Json<SearchResponse>) {
    let limit = req.limit.unwrap_or(10);

    let result = search_judgments(
        req.keyword.as_deref(),
        req.kenri.as_deref(),
        req.date_from.as_deref(),
        req.date_to.as_deref(),
        limit,
    )
    .await;

    match result {
        Ok(results) => (
            StatusCode::OK,
            Json(SearchResponse {
                success: true,
                results,
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SearchResponse {
                success: false,
                results: vec![],
                error: Some(e.to_string()),
            }),
        ),
    }
}

// PDF ダウンロード API
async fn download_pdf(Path(case_id): Path<u32>) -> Response {
    use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};

    let pdf_path = format!("/home/engineer/fast_crawler/output/ip_force_{}.pdf", case_id);

    match tokio::fs::read(&pdf_path).await {
        Ok(data) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/pdf")
                .header(CONTENT_DISPOSITION, "attachment; filename=\"report.pdf\"")
                .body(axum::body::Body::from(data))
                .unwrap()
        }
        Err(_) => {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(axum::body::Body::empty())
                .unwrap()
        }
    }
}

// 判決分析 API
async fn analyze(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AnalyzeRequest>,
) -> (StatusCode, Json<AnalyzeResponse>) {
    println!("📥 Received request: case_id={}", req.case_id);

    let result = process_case(&state, req.case_id).await;

    match result {
        Ok((title, case_no, pdf_path)) => (
            StatusCode::OK,
            Json(AnalyzeResponse {
                success: true,
                title: Some(title),
                case_no: Some(case_no),
                pdf_path: Some(pdf_path),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AnalyzeResponse {
                success: false,
                title: None,
                case_no: None,
                pdf_path: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

async fn process_case(state: &AppState, case_id: u32) -> Result<(String, String, String)> {
    // 1. IP Force から取得
    let mut patent = IpForcePatent::new(case_id);
    println!("📄 Fetching case {}...", case_id);
    let judgment_text = patent.fetch_and_extract().await?;
    println!("📄 Fetched {} chars", judgment_text.len());

    // 2. LLM で分析
    let system_prompt = patent.system_prompt();
    println!("🤖 Analyzing with LLM...");
    let llm_output = {
        let mut llm = state.llm.lock().await;
        llm.generate(&system_prompt, &judgment_text)?
    };
    println!("🤖 LLM done");

    // 3. JSON パース
    patent.load_llm_data(&llm_output)?;
    println!("📊 Parsed: {}", patent.title);

    // 4. Typst レンダリング
    let typst_source = patent.render()?;
    let output_dir = "/home/engineer/fast_crawler/output";
    std::fs::create_dir_all(output_dir)?;

    let typst_path = format!("{}/{}.typ", output_dir, patent.id());
    std::fs::write(&typst_path, &typst_source)?;

    // 5. PDF 生成
    let pdf_path = format!("{}/{}.pdf", output_dir, patent.id());
    let status = Command::new("typst")
        .args(["compile", &typst_path, &pdf_path])
        .status()?;

    if !status.success() {
        anyhow::bail!("Typst compile failed");
    }

    println!("✅ Generated: {}", pdf_path);
    Ok((patent.title.clone(), patent.case_no.clone(), pdf_path))
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== 🦀 My Legal Engine API Server ===\n");

    // LLM 初期化
    println!("⏳ Loading LLM...");
    let llm = LlmEngine::new().await?;
    println!("✅ LLM ready\n");

    let state = Arc::new(AppState {
        llm: Mutex::new(llm),
    });

    // CORS 設定
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // ルーティング
    let app = Router::new()
        .route("/health", get(health))
        .route("/analyze", post(analyze))
        .route("/search", post(search))
        .route("/pdf/:case_id", get(download_pdf))
        .fallback_service(ServeDir::new("frontend/dist"))
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:3000";
    println!("🚀 Server running on http://{}", addr);
    println!("   GET  /health");
    println!("   POST /search {{ \"keyword\": \"特許\", \"limit\": 10 }}");
    println!("   POST /analyze {{ \"case_id\": 14753 }}");
    println!("   GET  /pdf/14753\n");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
