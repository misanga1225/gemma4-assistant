pub mod types;
mod runner;
pub mod sandbox;
pub mod undo;
mod word;
mod excel;
mod powerpoint;

use std::path::PathBuf;
use types::{EditAction, EditResult};

#[derive(Clone)]
pub struct OfficeEditor {
    pub whitelist: Vec<PathBuf>,
    pub history_root: PathBuf,
    pub available: OfficeAvailability,
}

#[derive(Clone, Default, Debug)]
pub struct OfficeAvailability {
    pub word: bool,
    pub excel: bool,
    pub powerpoint: bool,
}

impl OfficeEditor {
    pub fn new(whitelist: Vec<PathBuf>, history_root: PathBuf) -> Self {
        Self {
            whitelist,
            history_root,
            available: OfficeAvailability::default(),
        }
    }

    pub async fn detect_availability(&mut self) {
        #[cfg(windows)]
        {
            self.available.word = word::is_available().await;
            self.available.excel = excel::is_available().await;
            self.available.powerpoint = powerpoint::is_available().await;
            eprintln!("[office] availability: {:?}", self.available);
        }
        #[cfg(not(windows))]
        {
            eprintln!("[office] 非Windows環境 — Office機能は無効");
        }
    }

    pub fn any_available(&self) -> bool {
        self.available.word || self.available.excel || self.available.powerpoint
    }

    pub async fn execute(&self, action: &EditAction) -> EditResult {
        #[cfg(not(windows))]
        {
            let _ = action;
            return EditResult::err("Windows環境のみサポート".to_string());
        }

        #[cfg(windows)]
        {
            let path_abs = match sandbox::check_allowed(action.path(), &self.whitelist) {
                Ok(p) => p,
                Err(e) => return EditResult::err(e),
            };

            if action.is_mutation() {
                if let Err(e) = undo::backup(&path_abs, &self.history_root) {
                    eprintln!("[office] バックアップ失敗(続行): {}", e);
                }
            }

            let p = path_abs.to_string_lossy().to_string();
            match action {
                EditAction::WordOpen { .. } => word::open(&p).await,
                EditAction::WordFindReplace { find, replace, match_case, .. } => {
                    word::find_replace(&p, find, replace, *match_case).await
                }
                EditAction::WordAppendParagraph { text, style, .. } => {
                    word::append_paragraph(&p, text, style.as_deref()).await
                }
                EditAction::WordInsertHeading { text, level, .. } => {
                    word::insert_heading(&p, text, *level).await
                }
                EditAction::WordSaveAs { dest, .. } => word::save_as(&p, dest).await,
                EditAction::ExcelOpen { .. } => excel::open(&p).await,
                EditAction::ExcelReadRange { sheet, range, .. } => {
                    excel::read_range(&p, sheet, range).await
                }
                EditAction::ExcelWriteCell { sheet, cell, value, .. } => {
                    excel::write_cell(&p, sheet, cell, value).await
                }
                EditAction::ExcelWriteRange { sheet, range, values, .. } => {
                    excel::write_range(&p, sheet, range, values).await
                }
                EditAction::ExcelAddFormula { sheet, cell, formula, .. } => {
                    excel::add_formula(&p, sheet, cell, formula).await
                }
                EditAction::PptxAddSlide { title, body, .. } => {
                    powerpoint::add_slide(&p, title.as_deref(), body.as_deref()).await
                }
                EditAction::PptxEditText { slide_index, shape_index, text, .. } => {
                    powerpoint::edit_text(&p, *slide_index, *shape_index, text).await
                }
            }
        }
    }

    pub fn undo(&self, target: &str) -> Result<PathBuf, String> {
        let abs = sandbox::check_allowed(target, &self.whitelist)?;
        undo::restore_latest(&abs, &self.history_root)
    }
}
