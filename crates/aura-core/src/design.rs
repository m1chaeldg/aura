//! Design system tokens and specifications for Aura, strictly implementing DESIGN.md.

/// Core hex color tokens from DESIGN.md
pub mod colors {
    pub const PRIMARY: &str = "#0066cc"; // Action Blue
    pub const PRIMARY_FOCUS: &str = "#0071e3"; // Focus Blue
    pub const PRIMARY_ON_DARK: &str = "#2997ff"; // Sky Link Blue
    pub const INK: &str = "#1d1d1f"; // Near-Black Ink
    pub const BODY: &str = "#1d1d1f"; // Body text
    pub const BODY_ON_DARK: &str = "#ffffff"; // Body On Dark
    pub const BODY_MUTED: &str = "#cccccc"; // Body Muted
    pub const INK_MUTED_80: &str = "#333333";
    pub const INK_MUTED_48: &str = "#7a7a7a";
    pub const DIVIDER_SOFT: &str = "#f0f0f0";
    pub const HAIRLINE: &str = "#e0e0e0";
    pub const CANVAS: &str = "#ffffff"; // Pure White
    pub const CANVAS_PARCHMENT: &str = "#f5f5f7"; // Signature Apple off-white
    pub const SURFACE_PEARL: &str = "#fafafc";
    pub const SURFACE_TILE_1: &str = "#272729"; // Dark Tile 1
    pub const SURFACE_TILE_2: &str = "#2a2a2c"; // Dark Tile 2
    pub const SURFACE_TILE_3: &str = "#252527"; // Dark Tile 3
    pub const SURFACE_BLACK: &str = "#000000"; // Pure Black
    pub const SURFACE_CHIP_TRANSLUCENT: &str = "#d2d2d7";

    // Standardized HTTP Method Badges from PRODUCT_SPEC.md & DESIGN.md
    pub const METHOD_GET: &str = "#0066cc"; // Blue
    pub const METHOD_POST: &str = "#28a745"; // Green
    pub const METHOD_PUT: &str = "#f5a623"; // Amber
    pub const METHOD_PATCH: &str = "#8e44ad"; // Purple
    pub const METHOD_DELETE: &str = "#e74c3c"; // Rose / Red

    // HTTP Status Badges
    pub const STATUS_2XX: &str = "#28a745"; // Green
    pub const STATUS_3XX: &str = "#0066cc"; // Blue
    pub const STATUS_4XX: &str = "#f5a623"; // Amber
    pub const STATUS_5XX: &str = "#e74c3c"; // Red

    // System Logging Levels
    pub const LOG_TRACE: &str = "#7a7a7a"; // Muted Ink
    pub const LOG_DEBUG: &str = "#333333"; // Secondary Ink
    pub const LOG_INFO: &str = "#0066cc"; // Action Blue
    pub const LOG_WARN: &str = "#f5a623"; // Amber
    pub const LOG_ERROR: &str = "#e74c3c"; // Rose / Red
}

/// Spacing scale (base unit: 8px)
pub mod spacing {
    pub const XXS: f32 = 4.0;
    pub const XS: f32 = 8.0;
    pub const SM: f32 = 12.0;
    pub const MD: f32 = 17.0;
    pub const LG: f32 = 24.0;
    pub const XL: f32 = 32.0;
    pub const XXL: f32 = 48.0;
    pub const SECTION: f32 = 80.0;
}

/// Border radius scale
pub mod rounded {
    pub const NONE: f32 = 0.0;
    pub const XS: f32 = 5.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 11.0;
    pub const LG: f32 = 18.0;
    pub const PILL: f32 = 9999.0;
}

/// Typography specifications
pub mod typography {
    pub const FONT_DISPLAY: &str = "SF Pro Display, system-ui, -apple-system, Inter, sans-serif";
    pub const FONT_TEXT: &str = "SF Pro Text, system-ui, -apple-system, Inter, sans-serif";

    pub struct Spec {
        pub font_family: &'static str,
        pub font_size: f32,
        pub font_weight: u16,
        pub line_height: f32,
        pub letter_spacing: f32,
    }

    pub const HERO_DISPLAY: Spec = Spec {
        font_family: FONT_DISPLAY,
        font_size: 56.0,
        font_weight: 600,
        line_height: 1.07,
        letter_spacing: -0.28,
    };

    pub const DISPLAY_LG: Spec = Spec {
        font_family: FONT_DISPLAY,
        font_size: 40.0,
        font_weight: 600,
        line_height: 1.10,
        letter_spacing: 0.0,
    };

    pub const DISPLAY_MD: Spec = Spec {
        font_family: FONT_TEXT,
        font_size: 34.0,
        font_weight: 600,
        line_height: 1.47,
        letter_spacing: -0.374,
    };

    pub const LEAD: Spec = Spec {
        font_family: FONT_DISPLAY,
        font_size: 28.0,
        font_weight: 400,
        line_height: 1.14,
        letter_spacing: 0.196,
    };

    pub const TAGLINE: Spec = Spec {
        font_family: FONT_DISPLAY,
        font_size: 21.0,
        font_weight: 600,
        line_height: 1.19,
        letter_spacing: 0.231,
    };

    pub const BODY_STRONG: Spec = Spec {
        font_family: FONT_TEXT,
        font_size: 17.0,
        font_weight: 600,
        line_height: 1.24,
        letter_spacing: -0.374,
    };

    pub const BODY: Spec = Spec {
        font_family: FONT_TEXT,
        font_size: 17.0,
        font_weight: 400,
        line_height: 1.47,
        letter_spacing: -0.374,
    };

    pub const CAPTION: Spec = Spec {
        font_family: FONT_TEXT,
        font_size: 14.0,
        font_weight: 400,
        line_height: 1.43,
        letter_spacing: -0.224,
    };

    pub const BUTTON_UTILITY: Spec = Spec {
        font_family: FONT_TEXT,
        font_size: 14.0,
        font_weight: 400,
        line_height: 1.29,
        letter_spacing: -0.224,
    };

    pub const FINE_PRINT: Spec = Spec {
        font_family: FONT_TEXT,
        font_size: 12.0,
        font_weight: 400,
        line_height: 1.0,
        letter_spacing: -0.12,
    };

    pub const NAV_LINK: Spec = Spec {
        font_family: FONT_TEXT,
        font_size: 12.0,
        font_weight: 400,
        line_height: 1.0,
        letter_spacing: -0.12,
    };
}
