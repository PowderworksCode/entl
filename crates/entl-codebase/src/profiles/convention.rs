use super::LanguageProfile;

#[derive(Debug, Clone, Copy)]
pub struct TestLayoutDefaults {
    pub source_roots: &'static [&'static str],
    pub test_root: &'static str,
    pub test_suffixes: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct TypecheckConvention {
    pub config_files: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageConventions {
    pub typecheck: Option<TypecheckConvention>,
    pub test_layout: TestLayoutDefaults,
    pub inline_test_detector: fn(&str) -> Option<&'static str>,
}

impl LanguageConventions {
    pub fn inline_test_indicator(&self, content: &str) -> Option<&'static str> {
        (self.inline_test_detector)(content)
    }
}

pub fn language_conventions(language: &LanguageProfile) -> Option<&LanguageConventions> {
    language.conventions.as_ref()
}
