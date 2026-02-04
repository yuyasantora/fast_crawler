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

    /// Typstソースコードをレンダリングして返す
    fn render(&self) -> Result<String>;
}

