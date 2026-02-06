/// プロンプト合成モジュール
///
/// テンプレート + 各コンポーネントを合成して最終プロンプトを生成
use anyhow::{Context, Result};
use std::fs;

/// プロンプトビルダー
pub struct PromptBuilder {
    template: String,
    attributes: Option<String>,
    knowledge: Option<String>,
    reasoning: Option<String>,
    task: Option<String>,
    schema: Option<String>,
}

impl PromptBuilder {
    /// テンプレートを読み込んで初期化
    pub fn new() -> Result<Self> {
        let template =
            fs::read_to_string("prompts/_template.md").context("Failed to read prompt template")?;

        Ok(Self {
            template,
            attributes: None,
            knowledge: None,
            reasoning: None,
            task: None,
            schema: None,
        })
    }

    /// Expert Attributes を設定
    pub fn with_attributes(mut self, path: &str) -> Result<Self> {
        self.attributes = Some(
            fs::read_to_string(path)
                .with_context(|| format!("Failed to read attributes: {}", path))?,
        );
        Ok(self)
    }

    /// Domain Knowledge を設定
    pub fn with_knowledge(mut self, path: &str) -> Result<Self> {
        self.knowledge = Some(
            fs::read_to_string(path)
                .with_context(|| format!("Failed to read knowledge: {}", path))?,
        );
        Ok(self)
    }

    /// Reasoning Framework を設定
    pub fn with_reasoning(mut self, path: &str) -> Result<Self> {
        self.reasoning = Some(
            fs::read_to_string(path)
                .with_context(|| format!("Failed to read reasoning: {}", path))?,
        );
        Ok(self)
    }

    /// Task Definition を設定
    pub fn with_task(mut self, path: &str) -> Result<Self> {
        self.task = Some(
            fs::read_to_string(path).with_context(|| format!("Failed to read task: {}", path))?,
        );
        Ok(self)
    }

    /// Output Schema を設定
    pub fn with_schema(mut self, path: &str) -> Result<Self> {
        self.schema = Some(
            fs::read_to_string(path).with_context(|| format!("Failed to read schema: {}", path))?,
        );
        Ok(self)
    }

    /// 最終プロンプトを生成
    pub fn build(self) -> String {
        let mut result = self.template;

        if let Some(attr) = self.attributes {
            result = result.replace("{{attributes}}", &attr);
        }
        if let Some(know) = self.knowledge {
            result = result.replace("{{knowledge}}", &know);
        }
        if let Some(reas) = self.reasoning {
            result = result.replace("{{reasoning}}", &reas);
        }
        if let Some(task) = self.task {
            result = result.replace("{{task}}", &task);
        }
        if let Some(schema) = self.schema {
            result = result.replace("{{schema}}", &schema);
        }

        result
    }
}

/// 特許判決分析用のプリセット
pub fn patent_judgment_prompt() -> Result<String> {
    let prompt = PromptBuilder::new()?
        .with_attributes("prompts/attributes/expert.md")?
        .with_knowledge("prompts/domains/patent/knowledge.md")?
        .with_reasoning("prompts/domains/patent/reasoning.md")?
        .with_task("prompts/tasks/patent/judgment_analysis.md")?
        .with_schema("prompts/tasks/patent/schemas/judgment.json")?
        .build();

    Ok(prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_builder() {
        let result = patent_judgment_prompt();
        assert!(result.is_ok());

        let prompt = result.unwrap();
        assert!(prompt.contains("Expert Attributes"));
        assert!(prompt.contains("Domain Knowledge"));
        assert!(prompt.contains("Reasoning Framework"));
    }
}
