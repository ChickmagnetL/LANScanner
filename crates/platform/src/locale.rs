#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemLanguage {
    Chinese,
    Other,
}

pub fn detect_system_language() -> SystemLanguage {
    classify_locale_identifier(current_locale_identifier().as_deref())
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn current_locale_identifier() -> Option<String> {
    sys_locale::get_locale()
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn current_locale_identifier() -> Option<String> {
    None
}

fn classify_locale_identifier(locale_identifier: Option<&str>) -> SystemLanguage {
    let Some(normalized) = normalize_locale_identifier(locale_identifier) else {
        return SystemLanguage::Other;
    };

    let Some(language) = normalized.split('-').next() else {
        return SystemLanguage::Other;
    };

    if language == "zh" {
        SystemLanguage::Chinese
    } else {
        SystemLanguage::Other
    }
}

fn normalize_locale_identifier(locale_identifier: Option<&str>) -> Option<String> {
    let trimmed = locale_identifier?.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_encoding = trimmed
        .split_once('.')
        .map_or(trimmed, |(prefix, _)| prefix);
    let without_modifier = without_encoding
        .split_once('@')
        .map_or(without_encoding, |(prefix, _)| prefix);

    let normalized = without_modifier.replace('_', "-").to_ascii_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::{SystemLanguage, classify_locale_identifier, normalize_locale_identifier};

    #[test]
    fn normalizes_common_locale_formats() {
        assert_eq!(
            normalize_locale_identifier(Some(" zh_CN.UTF-8 ")).as_deref(),
            Some("zh-cn")
        );
        assert_eq!(
            normalize_locale_identifier(Some("zh_Hant@calendar=roc")).as_deref(),
            Some("zh-hant")
        );
    }

    #[test]
    fn classifies_supported_chinese_markers_as_chinese() {
        for locale in [
            "zh",
            "zh-CN",
            "zh-SG",
            "zh-TW",
            "zh-HK",
            "zh-MO",
            "zh-Hans",
            "zh-Hant",
            "zh_CN.UTF-8",
            "zh_Hant@calendar=roc",
        ] {
            assert_eq!(
                classify_locale_identifier(Some(locale)),
                SystemLanguage::Chinese,
                "expected {locale} to map to Chinese"
            );
        }
    }

    #[test]
    fn classifies_non_chinese_and_missing_locales_as_other() {
        for locale in [
            None,
            Some(""),
            Some("en-US"),
            Some("ja-JP"),
            Some("fr_FR.UTF-8"),
        ] {
            assert_eq!(
                classify_locale_identifier(locale),
                SystemLanguage::Other,
                "expected {locale:?} to map to Other"
            );
        }
    }
}
