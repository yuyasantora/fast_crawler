use crate::summarizer::Summarizer;
use crate::traits::WebResource;
use anyhow::Result;
use askama::Template;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;

// --- Data Structures ---

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CaseInfo {
    pub title: String,
    pub case_no: String,
    pub court: String,
    pub date: String,
    pub result: String,
    pub patent_no: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IracIssue {
    pub plaintiff_claim: String,
    pub accused_product: String,
    pub key_disputes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IracRule {
    pub applicable_law: Vec<String>,
    pub claim_construction: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EquivalentRequirement {
    pub requirement: String,
    pub satisfied: bool,
    pub reasoning: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Equivalents {
    pub applied: bool,
    pub requirements: Vec<EquivalentRequirement>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InvalidityDefense {
    pub raised: bool,
    pub grounds: String,
    pub result: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IracAnalysis {
    pub infringement_type: String,
    pub equivalents: Option<Equivalents>,
    pub invalidity_defense: Option<InvalidityDefense>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IracConclusion {
    pub holding: String,
    pub damages: String,
    pub implications: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Irac {
    pub issue: IracIssue,
    pub rule: IracRule,
    pub analysis: IracAnalysis,
    pub conclusion: IracConclusion,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ClaimChartRow {
    pub element_id: String,
    pub claim_language: String,
    pub accused_feature: String,
    pub court_finding: String,
    pub satisfied: bool,
}

#[derive(Template, Serialize, Deserialize, Debug)]
#[template(path = "patent_report.typ", escape = "none")] // escape="none" はTypstに必須
pub struct IpForcePatent {
    // 内部管理用
    #[serde(skip)]
    pub case_id: u32,

    // LLM出力データ（IRAC構造）
    #[serde(default)]
    pub case_info: CaseInfo,
    #[serde(default)]
    pub irac: Irac,
    #[serde(default)]
    pub claim_chart: Vec<ClaimChartRow>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

impl IpForcePatent {
    pub fn new(case_id: u32) -> Self {
        Self {
            case_id,
            case_info: CaseInfo::default(),
            irac: Irac::default(),
            claim_chart: vec![],
            keywords: vec![],
            summary: String::new(),
        }
    }

    /// パース済みデータからフィールドを更新
    pub fn update_from_parsed(&mut self, data: IpForcePatent) {
        self.case_info = data.case_info;
        self.irac = data.irac;
        self.claim_chart = data.claim_chart;
        self.keywords = data.keywords;
        self.summary = data.summary;
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
    kenri: Option<&str>,
    date_from: Option<&str>,  // YYYY-MM形式
    date_to: Option<&str>,    // YYYY-MM形式
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let client = Client::new();
    let url = "https://ipforce.jp/Hanketsu/search";

    println!("🔍 Searching: {} (butsu={}, kenri={}, from={}, to={})",
        url, keyword.unwrap_or(""), kenri.unwrap_or("all"),
        date_from.unwrap_or(""), date_to.unwrap_or(""));

    // POST リクエストで検索
    let mut form = vec![
        ("ruikei[]", "0"),  // 民事
        ("ruikei[]", "1"),  // 審決取消訴訟
    ];

    let keyword_owned: String;
    if let Some(kw) = keyword {
        keyword_owned = kw.to_string();
        form.push(("butsu", &keyword_owned));
    }

    let kenri_owned: String;
    if let Some(k) = kenri {
        kenri_owned = k.to_string();
        form.push(("kenri", &kenri_owned));
    }

    // 日付範囲（YYYY-MM形式をパース）
    let (sby, sbm): (String, String);
    if let Some(from) = date_from {
        if let Some((y, m)) = from.split_once('-') {
            sby = y.to_string();
            sbm = m.to_string();
            form.push(("sbY", &sby));
            form.push(("sbM", &sbm));
        }
    }

    let (sbyl, sbml): (String, String);
    if let Some(to) = date_to {
        if let Some((y, m)) = to.split_once('-') {
            sbyl = y.to_string();
            sbml = m.to_string();
            form.push(("sbYl", &sbyl));
            form.push(("sbMl", &sbml));
        }
    }

    let body = client.post(url).form(&form).send().await?.text().await?;
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

        // 論理的読解法ベースの要約で重要文を抽出
        let safe_length = 6000;
        let extracted = match Summarizer::new() {
            Ok(summarizer) => {
                match summarizer.extract(&text, safe_length) {
                    Ok(summary) => {
                        println!("📝 Summarized: {} → {} chars", text.chars().count(), summary.chars().count());
                        summary
                    }
                    Err(e) => {
                        println!("⚠️ Summarization failed, falling back to truncation: {}", e);
                        text.chars().take(safe_length).collect()
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Summarizer init failed, falling back to truncation: {}", e);
                text.chars().take(safe_length).collect()
            }
        };

        Ok(extracted)
    }

    fn system_prompt(&self) -> String {
        crate::prompt::patent_judgment_prompt()
            .unwrap_or_else(|_| {
                fs::read_to_string("prompts/ip_force.md")
                    .unwrap_or_else(|_| "You are a helpful assistant.".to_string())
            })
    }

    fn render(&self) -> Result<String> {
        Ok(askama::Template::render(self)?)
    }
}
