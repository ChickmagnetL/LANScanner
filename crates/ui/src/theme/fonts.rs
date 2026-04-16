use iced::Font;

pub const BUNDLED_CJK_FAMILY: &str = "Noto Sans SC";
#[cfg(target_os = "windows")]
const WINDOWS_CJK_FAMILY: &str = "Microsoft YaHei UI";

pub fn body() -> Font {
    #[cfg(target_os = "windows")]
    {
        // 全局中文必须绑定到可覆盖 CJK 的确定字体，不能依赖 Font::DEFAULT 的不确定回退。
        Font::with_name(WINDOWS_CJK_FAMILY)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Font::with_name(BUNDLED_CJK_FAMILY)
    }
}

pub fn body_alt() -> Font {
    #[cfg(target_os = "windows")]
    {
        Font::with_name("Segoe UI")
    }

    #[cfg(not(target_os = "windows"))]
    {
        body()
    }
}

pub fn icon() -> Font {
    #[cfg(target_os = "windows")]
    {
        // Keep icon glyphs independent from CJK body text font.
        Font::with_name("Segoe UI Symbol")
    }

    #[cfg(not(target_os = "windows"))]
    {
        body_alt()
    }
}

pub fn semibold() -> Font {
    body()
}

pub fn icon_semibold() -> Font {
    icon()
}
