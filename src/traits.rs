use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait WebResource {
    /// 一意な識別子（ファイル名などに使用）
    fn id(&self) -> String;

    /// ターゲットURLを取得し、テキスト抽出を行う
    async fn fetch_and_extract(&self) -> Result<String>;

    /// システムプロンプトを取得する
    fn system_prompt(&self) -> String;

    /// LLMの出力結果(JSON文字列)を自身の構造体にロードする
    fn load_llm_data(&mut self, llm_output: &str) -> Result<()>;

    /// Typstソースコードをレンダリングして返す
    fn render(&self) -> Result<String>;
}

