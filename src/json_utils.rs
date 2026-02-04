/// JSON抽出・修復ユーティリティ

/// LLM出力からJSON部分を抽出
pub fn extract_json(text: &str) -> String {
    // ```json ... ``` ブロックがあれば抽出
    if let Some(start) = text.find("```json") {
        let after_marker = &text[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }

    // { ... } を探す
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }

    text.to_string()
}

/// よくあるJSON構文エラーを修復
pub fn repair_json(json: &str) -> String {
    let mut result = json.to_string();

    // 1. trailing comma を削除: ,] → ] , ,} → }
    let trailing_comma = regex::Regex::new(r",(\s*[}\]])").unwrap();
    result = trailing_comma.replace_all(&result, "$1").to_string();

    // 2. 制御文字を除去（改行・タブ以外）
    result = result
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t' || *c == '\r')
        .collect();

    // 3. 不正なエスケープを修正（よくあるパターン）
    result = result.replace("\\'", "'");

    // 4. 括弧の数をチェックして補完
    let open_braces = result.matches('{').count();
    let close_braces = result.matches('}').count();
    if open_braces > close_braces {
        for _ in 0..(open_braces - close_braces) {
            result.push('}');
        }
    }

    let open_brackets = result.matches('[').count();
    let close_brackets = result.matches(']').count();
    if open_brackets > close_brackets {
        for _ in 0..(open_brackets - close_brackets) {
            result.push(']');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_markdown() {
        let input = "Here is the result:\n```json\n{\"key\": \"value\"}\n```\nDone.";
        assert_eq!(extract_json(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_extract_json_raw() {
        let input = "Result: {\"key\": \"value\"} end";
        assert_eq!(extract_json(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_repair_trailing_comma() {
        let input = r#"{"items": ["a", "b",], "x": 1,}"#;
        let repaired = repair_json(input);
        assert!(serde_json::from_str::<serde_json::Value>(&repaired).is_ok());
    }

    #[test]
    fn test_repair_missing_brace() {
        let input = r#"{"nested": {"a": 1}"#;
        let repaired = repair_json(input);
        assert!(repaired.ends_with('}'));
    }
}
