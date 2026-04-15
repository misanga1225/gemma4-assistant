use crate::office::OfficeAvailability;

/// Gemma4にツール使用方法を伝えるシステムプロンプト断片を生成。
pub fn tool_system_prompt(avail: &OfficeAvailability) -> String {
    if !avail.word && !avail.excel && !avail.powerpoint {
        return String::new();
    }

    let mut tools: Vec<&str> = Vec::new();
    if avail.word {
        tools.push(r#"- word_open {"path": "..."}                      Word文書を開く/新規作成
- word_find_replace {"path","find","replace","match_case":false}      置換
- word_append_paragraph {"path","text","style":"..."}                 段落追加
- word_insert_heading {"path","text","level":1}                       見出し挿入
- word_save_as {"path","dest"}                                        名前を付けて保存"#);
    }
    if avail.excel {
        tools.push(r#"- excel_open {"path"}                                                  ブックを開く
- excel_read_range {"path","sheet","range"}                            範囲読み取り
- excel_write_cell {"path","sheet","cell","value"}                     セル書き込み
- excel_write_range {"path","sheet","range","values":[["..",".."]]}    範囲書き込み
- excel_add_formula {"path","sheet","cell","formula":"=SUM(A1:A3)"}    数式挿入"#);
    }
    if avail.powerpoint {
        tools.push(r#"- pptx_add_slide {"path","title":"...","body":"..."}                   スライド追加
- pptx_edit_text {"path","slide_index":1,"shape_index":1,"text":"..."}テキスト編集"#);
    }

    format!(
        r#"

【Office編集ツール】
ユーザーがWord / Excel / PowerPointの編集を依頼したときのみ、以下のフォーマットで**1回につき1ツール**を呼び出せます。

```tool
{{"tool": "<tool_name>", "args": {{...}}}}
```

利用可能ツール:
{}

ルール:
- 通常のおしゃべりでは絶対にtoolブロックを出さないこと。
- toolブロックを出すときは、その直前にキャラクターとして一言短く宣言（例：「はいはい、やってあげるわよ」）。
- pathは絶対パスで指定する。ユーザーが相対パスを言ったら「どのフォルダ？」と聞き返すこと。
- ツール実行結果はsystemメッセージとして戻される。その結果を元に最終返答すること。
"#,
        tools.join("\n")
    )
}
