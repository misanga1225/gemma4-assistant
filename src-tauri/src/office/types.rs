use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum EditAction {
    WordOpen { path: String },
    WordFindReplace {
        path: String,
        find: String,
        replace: String,
        #[serde(default)]
        match_case: bool,
    },
    WordAppendParagraph {
        path: String,
        text: String,
        #[serde(default)]
        style: Option<String>,
    },
    WordInsertHeading {
        path: String,
        text: String,
        level: u8,
    },
    WordSaveAs {
        path: String,
        dest: String,
    },
    ExcelOpen { path: String },
    ExcelReadRange {
        path: String,
        sheet: String,
        range: String,
    },
    ExcelWriteCell {
        path: String,
        sheet: String,
        cell: String,
        value: String,
    },
    ExcelWriteRange {
        path: String,
        sheet: String,
        range: String,
        values: Vec<Vec<String>>,
    },
    ExcelAddFormula {
        path: String,
        sheet: String,
        cell: String,
        formula: String,
    },
    PptxAddSlide {
        path: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        body: Option<String>,
    },
    PptxEditText {
        path: String,
        slide_index: i32,
        shape_index: i32,
        text: String,
    },
}

impl EditAction {
    pub fn path(&self) -> &str {
        match self {
            EditAction::WordOpen { path }
            | EditAction::WordFindReplace { path, .. }
            | EditAction::WordAppendParagraph { path, .. }
            | EditAction::WordInsertHeading { path, .. }
            | EditAction::WordSaveAs { path, .. }
            | EditAction::ExcelOpen { path }
            | EditAction::ExcelReadRange { path, .. }
            | EditAction::ExcelWriteCell { path, .. }
            | EditAction::ExcelWriteRange { path, .. }
            | EditAction::ExcelAddFormula { path, .. }
            | EditAction::PptxAddSlide { path, .. }
            | EditAction::PptxEditText { path, .. } => path,
        }
    }

    pub fn is_mutation(&self) -> bool {
        !matches!(
            self,
            EditAction::WordOpen { .. }
                | EditAction::ExcelOpen { .. }
                | EditAction::ExcelReadRange { .. }
        )
    }

    pub fn tool_name(&self) -> &'static str {
        match self {
            EditAction::WordOpen { .. } => "word_open",
            EditAction::WordFindReplace { .. } => "word_find_replace",
            EditAction::WordAppendParagraph { .. } => "word_append_paragraph",
            EditAction::WordInsertHeading { .. } => "word_insert_heading",
            EditAction::WordSaveAs { .. } => "word_save_as",
            EditAction::ExcelOpen { .. } => "excel_open",
            EditAction::ExcelReadRange { .. } => "excel_read_range",
            EditAction::ExcelWriteCell { .. } => "excel_write_cell",
            EditAction::ExcelWriteRange { .. } => "excel_write_range",
            EditAction::ExcelAddFormula { .. } => "excel_add_formula",
            EditAction::PptxAddSlide { .. } => "pptx_add_slide",
            EditAction::PptxEditText { .. } => "pptx_edit_text",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EditResult {
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl EditResult {
    pub fn ok(msg: String) -> Self {
        Self { ok: true, message: msg, data: None }
    }
    pub fn ok_data(msg: String, data: serde_json::Value) -> Self {
        Self { ok: true, message: msg, data: Some(data) }
    }
    pub fn err(msg: String) -> Self {
        Self { ok: false, message: msg, data: None }
    }
}
