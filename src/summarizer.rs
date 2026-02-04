/// 論理的読解法ベースのテキスト要約モジュール
///
/// 形態素解析で接続詞を検出し、論理構造に基づいて重要文を抽出

use anyhow::Result;
use lindera::segmenter::{Segmenter, SegmenterConfig};
use lindera::tokenizer::Tokenizer;
use std::collections::HashMap;

/// 要約器
pub struct Summarizer {
    tokenizer: Tokenizer,
}

impl Summarizer {
    /// 新しいSummarizerを作成
    pub fn new() -> Result<Self> {
        let config: SegmenterConfig = serde_json::from_value(serde_json::json!({
            "dictionary": "embedded://ipadic",
        }))?;
        let segmenter = Segmenter::from_config(&config)?;
        let tokenizer = Tokenizer::new(segmenter);
        Ok(Self { tokenizer })
    }

    /// 重要文を抽出
    pub fn extract(&self, text: &str, max_chars: usize) -> Result<String> {
        // 1. 文分割
        let sentences = self.split_sentences(text);

        if sentences.is_empty() {
            return Ok(text.chars().take(max_chars).collect());
        }

        // 2. 頻出語を抽出
        let word_freq = self.count_word_frequency(text)?;

        // 3. 各文にスコアを付与
        let scored: Vec<(usize, &str, f64)> = sentences
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let score = self.calculate_score(s, i, sentences.len(), &word_freq);
                (i, *s, score)
            })
            .collect();

        // 4. スコア上位を選択（元の順序は保持）
        let mut selected = self.select_top_sentences(&scored, max_chars);
        selected.sort_by_key(|(i, _, _)| *i);

        // 5. 結合して返す
        let result = selected
            .into_iter()
            .map(|(_, s, _)| s)
            .collect::<Vec<_>>()
            .join("");

        Ok(result)
    }

    /// 文分割
    fn split_sentences<'a>(&self, text: &'a str) -> Vec<&'a str> {
        let mut sentences = Vec::new();
        let mut start = 0;

        for (i, c) in text.char_indices() {
            if c == '。' || c == '！' || c == '？' {
                let end = i + c.len_utf8();
                let sentence = &text[start..end];
                if !sentence.trim().is_empty() {
                    sentences.push(sentence);
                }
                start = end;
            }
        }

        // 最後の文（句点なし）
        if start < text.len() {
            let remaining = &text[start..];
            if !remaining.trim().is_empty() {
                sentences.push(remaining);
            }
        }

        sentences
    }

    /// 単語の出現頻度をカウント（形態素解析）
    fn count_word_frequency(&self, text: &str) -> Result<HashMap<String, usize>> {
        let mut freq = HashMap::new();

        let mut tokens = self.tokenizer.tokenize(text)?;

        for token in &mut tokens {
            let word = token.surface.to_string();
            // 2文字以上の名詞・動詞・形容詞をカウント
            if word.chars().count() >= 2 {
                let details = token.details();
                let pos = details.first().copied().unwrap_or("");
                if pos == "名詞" || pos == "動詞" || pos == "形容詞" {
                    *freq.entry(word).or_insert(0) += 1;
                }
            }
        }

        Ok(freq)
    }

    /// 文のスコアを計算
    fn calculate_score(
        &self,
        sentence: &str,
        index: usize,
        total_sentences: usize,
        word_freq: &HashMap<String, usize>,
    ) -> f64 {
        let mut score = 0.0;

        // 1. 接続詞スコア（形態素解析で検出）
        score += self.connector_score(sentence);

        // 2. 頻出語スコア
        score += self.frequency_score(sentence, word_freq);

        // 3. 位置スコア
        score += self.position_score(index, total_sentences);

        score
    }

    /// 接続詞による重要度スコア（形態素解析で検出）
    fn connector_score(&self, sentence: &str) -> f64 {
        let mut tokens = match self.tokenizer.tokenize(sentence) {
            Ok(t) => t,
            Err(_) => return 0.0,
        };

        // 文頭の数トークンをチェック
        for (i, token) in tokens.iter_mut().enumerate() {
            if i > 3 {
                break; // 文頭付近のみチェック
            }

            let details = token.details();
            let pos = details.first().copied().unwrap_or("");

            if pos == "接続詞" {
                let word: &str = token.surface.as_ref();

                // 結論系の接続詞（高スコア）
                if matches!(
                    word,
                    "したがって" | "よって" | "ゆえに" | "つまり" | "すなわち" | "結局"
                ) {
                    return 3.0;
                }

                // 転換・対比系（中スコア）
                if matches!(
                    word,
                    "しかし" | "ところが" | "だが" | "けれども" | "一方"
                ) {
                    return 2.0;
                }

                // その他の接続詞
                return 1.5;
            }
        }

        0.0
    }

    /// 頻出語による重要度スコア
    fn frequency_score(&self, sentence: &str, word_freq: &HashMap<String, usize>) -> f64 {
        let tokens = match self.tokenizer.tokenize(sentence) {
            Ok(t) => t,
            Err(_) => return 0.0,
        };

        let mut score = 0.0;

        for token in tokens {
            if let Some(&freq) = word_freq.get(&token.surface.to_string()) {
                if freq >= 2 {
                    score += (freq as f64).ln();
                }
            }
        }

        score.min(3.0)
    }

    /// 位置による重要度スコア
    fn position_score(&self, index: usize, total: usize) -> f64 {
        if total == 0 {
            return 0.0;
        }

        let position_ratio = index as f64 / total as f64;

        if index == 0 {
            return 1.5; // 冒頭
        }

        if index == total - 1 {
            return 2.0; // 末尾
        }

        if position_ratio > 0.7 {
            return 1.0; // 後半
        }

        0.0
    }

    /// スコア上位の文を選択
    fn select_top_sentences<'a>(
        &self,
        scored: &[(usize, &'a str, f64)],
        max_chars: usize,
    ) -> Vec<(usize, &'a str, f64)> {
        let mut sorted: Vec<_> = scored.to_vec();
        sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        let mut selected = Vec::new();
        let mut total_chars = 0;

        for item in sorted {
            let chars = item.1.chars().count();
            if total_chars + chars <= max_chars {
                selected.push(item);
                total_chars += chars;
            }

            if total_chars >= max_chars {
                break;
            }
        }

        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarizer() -> Result<()> {
        let summarizer = Summarizer::new()?;

        let text = "これは導入文です。しかし、重要な問題がある。したがって、対策が必要である。補足説明です。";

        let result = summarizer.extract(text, 100)?;
        println!("Result: {}", result);

        // 接続詞を含む文が優先されるはず
        assert!(result.contains("したがって") || result.contains("しかし"));

        Ok(())
    }
}
