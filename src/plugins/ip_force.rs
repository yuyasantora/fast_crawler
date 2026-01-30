use crate::traits::WebResource;
use anyhow::{Context, Result};
use askama::Template;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;

// --- Data Structures ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClaimRow {
    pub requirement: String,
    pub defendant: String,
    pub judgment: String,
    pub is_satisfied: bool,
}

#[derive(Template, Serialize, Deserialize, Debug)]
#[template(path = "patent_report.typ", escape = "none")] // escape="none" はTypstに必須
pub struct IpForcePatent {
    // 内部管理用
    #[serde(skip)]
    pub case_id: u32,

    // LLM出力データ
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub case_no: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub claim_chart: Vec<ClaimRow>,
}

impl IpForcePatent {
    pub fn new(case_id: u32) -> Self {
        Self {
            case_id,
            title: String::new(),
            case_no: String::new(),
            date: String::new(),
            result: String::new(),
            summary: String::new(),
            keywords: vec![],
            claim_chart: vec![],
        }
    }
}

// --- Search ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchResult {
    pub case_id: u32,
    pub title: String,
    pub date: String,
}

pub async fn search_judgments(
    keyword: Option<&str>,
    _kenri: Option<&str>,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let client = Client::new();

    // URL 構築 - IP Force のサーバー側検索を使用
    let url = match keyword {
        Some(kw) => format!(
            "https://ipforce.jp/Hanketsu/search/keyword/{}",
            urlencoding::encode(kw)
        ),
        None => "https://ipforce.jp/Hanketsu/search".to_string(),
    };

    println!("🔍 Searching: {}", url);

    let body = client.get(&url).send().await?.text().await?;
    let document = Html::parse_document(&body);

    let link_selector = Selector::parse("span.name a[href*='/Hanketsu/jiken/no/']").unwrap();

    let mut results = Vec::new();

    for elem in document.select(&link_selector) {
        if results.len() >= limit {
            break;
        }

        // case_id を URL から抽出
        if let Some(href) = elem.value().attr("href") {
            if let Some(id_str) = href.split("/no/").nth(1) {
                if let Ok(case_id) = id_str.trim_matches('/').parse::<u32>() {
                    let title = elem.text().collect::<Vec<_>>().join("").trim().to_string();

                    if title.is_empty() {
                        continue;
                    }

                    results.push(SearchResult {
                        case_id,
                        title,
                        date: String::new(),
                    });
                }
            }
        }
    }

    Ok(results)
}

// --- Trait Implementation ---

#[async_trait]
impl WebResource for IpForcePatent {
    fn id(&self) -> String {
        format!("ip_force_{}", self.case_id)
    }

    async fn fetch_and_extract(&self) -> Result<String> {
        let url = format!("https://ipforce.jp/Hanketsu/jiken/no/{}", self.case_id);
        println!("🌐 Fetching: {}", url);

        let client = Client::new();
        let body = client.get(&url).send().await?.text().await?;

        let document = Html::parse_document(&body);

        // IP Forceの構造に合わせてセレクタを指定
        // 本文が入っている可能性が高いID
        let selector = Selector::parse("div#hanketsu_contents").unwrap();

        let text = if let Some(elem) = document.select(&selector).next() {
            elem.text().collect::<Vec<_>>().join("")
        } else {
            // 見つからない場合はbody全体から取得（フォールバック）
            document.root_element().text().collect::<Vec<_>>().join(" ")
        };

        // LLMのトークン制限を考慮して、適当な長さに切り詰める
        // (本来はもっと賢い分割処理が必要)
        let safe_length = 5000;
        let truncated: String = text.chars().take(safe_length).collect();

        Ok(truncated)
    }

    fn system_prompt(&self) -> String {
        // 実行時にファイルから読み込む
        fs::read_to_string("prompts/ip_force.md")
            .unwrap_or_else(|_| "You are a helpful assistant.".to_string())
    }

    fn load_llm_data(&mut self, llm_output: &str) -> Result<()> {
        // ```json ... ``` からJSON部分を抽出
        let json_str = if let Some(start) = llm_output.find('{') {
            if let Some(end) = llm_output.rfind('}') {
                &llm_output[start..=end]
            } else {
                llm_output
            }
        } else {
            llm_output
        };

        let data: IpForcePatent =
            serde_json::from_str(json_str).context("Failed to parse LLM JSON output")?;

        // 自身のフィールドを更新
        self.title = data.title;
        self.case_no = data.case_no;
        self.date = data.date;
        self.result = data.result;
        self.summary = data.summary;
        self.keywords = data.keywords;
        self.claim_chart = data.claim_chart;

        Ok(())
    }

    fn render(&self) -> Result<String> {
        Ok(askama::Template::render(self)?)
    }
}
