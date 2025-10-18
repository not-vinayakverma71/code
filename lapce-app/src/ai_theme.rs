/// AI Theme - Comprehensive Windsurf static colors
///
/// Extracted from windsurf.css and small.html
/// ONLY includes static (hardcoded) colors, NOT VS Code theme variables

use floem::peniko::Color;

#[derive(Debug, Clone)]
pub struct AiTheme {
    // Typography
    pub font_family: &'static str,
    pub editor_font_family: &'static str,
    pub font_size: f32,
    pub editor_font_size: f32,
    pub line_height: f32,

    // Base colors
    pub foreground: Color,
    pub background: Color,
    pub editor_background: Color,

    // AI-specific colors (dim text when AI talks)
    pub description_foreground: Color,
    pub ai_text_medium: Color,          // 55% opacity for dark, 80% for light
    pub ai_text_light: Color,           // 35% opacity for dark, 50% for light

    // Chat container
    pub chat_background: Color,         // #202020
    pub chat_border: Color,             // #454545
    pub chat_shadow: Color,             // rgba(0,0,0,0.36)

    // Input box
    pub input_background: Color,        // #313131
    pub input_foreground: Color,        // #cccccc
    pub input_border: Color,            // #3c3c3c
    pub input_focus_border: Color,      // #0078d4 (blue accent)
    pub input_placeholder: Color,       // #989898

    // Messages
    pub message_user_background: Color, // rgba(31,31,31,0.62)
    pub message_user_foreground: Color,
    pub message_bot_background: Color,  // #1f1f1f
    pub message_bot_foreground: Color,  // dimmed
    pub message_border: Color,          // rgba(255,255,255,0.1)

    // Special elements
    pub command_background: Color,      // #34414b (blue-gray)
    pub command_foreground: Color,      // #40a6ff (bright blue)
    pub avatar_background: Color,       // #1f1f1f

    // Code blocks
    pub code_block_background: Color,   // #202020
    pub inline_code_background: Color,  // #313131
    pub inline_code_foreground: Color,

    // Buttons
    pub button_primary: Color,          // #0078d4
    pub button_primary_hover: Color,    // #026ec1
    pub button_primary_foreground: Color, // #ffffff
    pub button_secondary: Color,        // #313131
    pub button_secondary_hover: Color,  // #3c3c3c
    pub button_secondary_foreground: Color,

    // Panel styling
    pub panel_bg: Color,
    pub panel_border: Color,
    pub panel_shadow: Color,

    // Interactive elements
    pub hover_background: Color,
    pub active_background: Color,
}

impl Default for AiTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl AiTheme {
    /// Dark theme with comprehensive Windsurf static colors
    pub fn dark() -> Self {
        let foreground = Color::from_rgb8(0xcc, 0xcc, 0xcc); // #cccccc
        let editor_bg = Color::from_rgb8(0x1f, 0x1f, 0x1f);  // #1f1f1f

        Self {
            // Typography
            font_family: "system-ui, Ubuntu, Droid Sans, sans-serif",
            editor_font_family: "Droid Sans Mono, Liberation Mono, DejaVu Sans Mono, Courier New, monospace",
            font_size: 13.0,
            editor_font_size: 14.0,
            line_height: 1.5,

            // Base colors
            foreground,
            background: editor_bg,
            editor_background: editor_bg,

            // AI text dimming
            description_foreground: Color::from_rgb8(0x9d, 0x9d, 0x9d),
            ai_text_medium: foreground.multiply_alpha(0.55),
            ai_text_light: foreground.multiply_alpha(0.35),

            // Chat container - static colors from windsurf.css
            chat_background: Color::from_rgb8(0x20, 0x20, 0x20),      // #202020
            chat_border: Color::from_rgb8(0x45, 0x45, 0x45),          // #454545
            chat_shadow: Color::from_rgba8(0, 0, 0, 92),              // rgba(0,0,0,0.36)

            // Input box - static colors
            input_background: Color::from_rgb8(0x31, 0x31, 0x31),     // #313131
            input_foreground: Color::from_rgb8(0xcc, 0xcc, 0xcc),     // #cccccc
            input_border: Color::from_rgb8(0x3c, 0x3c, 0x3c),         // #3c3c3c
            input_focus_border: Color::from_rgb8(0x00, 0x78, 0xd4),   // #0078d4
            input_placeholder: Color::from_rgb8(0x98, 0x98, 0x98),    // #989898

            // Messages
            message_user_background: Color::from_rgba8(31, 31, 31, 158), // rgba(31,31,31,0.62)
            message_user_foreground: foreground,
            message_bot_background: Color::from_rgb8(0x1f, 0x1f, 0x1f), // #1f1f1f
            message_bot_foreground: foreground.multiply_alpha(0.55),
            message_border: Color::from_rgba8(255, 255, 255, 26),     // rgba(255,255,255,0.1)

            // Special elements
            command_background: Color::from_rgb8(0x34, 0x41, 0x4b),   // #34414b
            command_foreground: Color::from_rgb8(0x40, 0xa6, 0xff),   // #40a6ff
            avatar_background: Color::from_rgb8(0x1f, 0x1f, 0x1f),    // #1f1f1f

            // Code blocks - static colors
            code_block_background: Color::from_rgb8(0x20, 0x20, 0x20), // #202020
            inline_code_background: Color::from_rgb8(0x31, 0x31, 0x31), // #313131
            inline_code_foreground: foreground,

            // Buttons - static colors
            button_primary: Color::from_rgb8(0x00, 0x78, 0xd4),       // #0078d4
            button_primary_hover: Color::from_rgb8(0x02, 0x6e, 0xc1), // #026ec1
            button_primary_foreground: Color::from_rgb8(0xff, 0xff, 0xff), // #ffffff
            button_secondary: Color::from_rgb8(0x31, 0x31, 0x31),     // #313131
            button_secondary_hover: Color::from_rgb8(0x3c, 0x3c, 0x3c), // #3c3c3c
            button_secondary_foreground: Color::from_rgb8(0xcc, 0xcc, 0xcc),

            // Panel styling
            panel_bg: Self::color_mix(editor_bg, Color::from_rgb8(0x20, 0x20, 0x20), 0.5),
            panel_border: Self::color_mix(foreground, editor_bg, 0.05),
            panel_shadow: Color::from_rgba8(0, 0, 0, 38),

            // Interactive
            hover_background: Color::from_rgba8(0x73, 0x73, 0x73, 26),
            active_background: Color::from_rgba8(0x73, 0x73, 0x73, 51),
        }
    }

    /// Light theme with Windsurf light mode colors
    pub fn light() -> Self {
        let foreground = Color::from_rgb8(0x33, 0x33, 0x33);
        let editor_bg = Color::from_rgb8(0xff, 0xff, 0xff);

        Self {
            font_family: "system-ui, Ubuntu, Droid Sans, sans-serif",
            editor_font_family: "Droid Sans Mono, Liberation Mono, DejaVu Sans Mono, Courier New, monospace",
            font_size: 13.0,
            editor_font_size: 14.0,
            line_height: 1.5,

            foreground,
            background: editor_bg,
            editor_background: editor_bg,

            description_foreground: Color::from_rgb8(0x71, 0x71, 0x71),
            ai_text_medium: foreground.multiply_alpha(0.80),
            ai_text_light: foreground.multiply_alpha(0.50),

            // Chat container - lighter versions
            chat_background: Color::from_rgb8(0xf5, 0xf5, 0xf5),
            chat_border: Color::from_rgb8(0xd0, 0xd0, 0xd0),
            chat_shadow: Color::from_rgba8(0, 0, 0, 19),

            // Input box - light theme
            input_background: Color::from_rgb8(0xff, 0xff, 0xff),
            input_foreground: Color::from_rgb8(0x33, 0x33, 0x33),
            input_border: Color::from_rgb8(0xd0, 0xd0, 0xd0),
            input_focus_border: Color::from_rgb8(0x00, 0x78, 0xd4),
            input_placeholder: Color::from_rgb8(0x98, 0x98, 0x98),

            // Messages - light theme
            message_user_background: Color::from_rgba8(220, 220, 220, 128),
            message_user_foreground: foreground,
            message_bot_background: Color::from_rgb8(0xf5, 0xf5, 0xf5),
            message_bot_foreground: foreground.multiply_alpha(0.80),
            message_border: Color::from_rgba8(0, 0, 0, 26),

            // Special elements - light theme
            command_background: Color::from_rgb8(0xd0, 0xe0, 0xf0),
            command_foreground: Color::from_rgb8(0x00, 0x78, 0xd4),
            avatar_background: Color::from_rgb8(0xe0, 0xe0, 0xe0),

            // Code blocks - light theme
            code_block_background: Color::from_rgb8(0xf5, 0xf5, 0xf5),
            inline_code_background: Color::from_rgb8(0xe8, 0xe8, 0xe8),
            inline_code_foreground: foreground,

            // Buttons - same as dark (blue accent)
            button_primary: Color::from_rgb8(0x00, 0x78, 0xd4),
            button_primary_hover: Color::from_rgb8(0x02, 0x6e, 0xc1),
            button_primary_foreground: Color::from_rgb8(0xff, 0xff, 0xff),
            button_secondary: Color::from_rgb8(0xf0, 0xf0, 0xf0),
            button_secondary_hover: Color::from_rgb8(0xe0, 0xe0, 0xe0),
            button_secondary_foreground: foreground,

            panel_bg: editor_bg,
            panel_border: Color::from_rgba8(0, 0, 0, 26),
            panel_shadow: Color::from_rgba8(0, 0, 0, 19),

            hover_background: Color::from_rgba8(0, 0, 0, 13),
            active_background: Color::from_rgba8(0, 0, 0, 26),
        }
    }

    /// Simple color-mix implementation (like CSS color-mix)
    fn color_mix(a: Color, b: Color, ratio: f32) -> Color {
        let a_rgba = a.to_rgba8();
        let b_rgba = b.to_rgba8();
        
        let r = (a_rgba.r as f32 * ratio + b_rgba.r as f32 * (1.0 - ratio)) as u8;
        let g = (a_rgba.g as f32 * ratio + b_rgba.g as f32 * (1.0 - ratio)) as u8;
        let b_val = (a_rgba.b as f32 * ratio + b_rgba.b as f32 * (1.0 - ratio)) as u8;
        let a_val = (a_rgba.a as f32 * ratio + b_rgba.a as f32 * (1.0 - ratio)) as u8;
        
        Color::from_rgba8(r, g, b_val, a_val)
    }
}

/// Spacing constants from Windsurf CSS
pub mod spacing {
    // Padding values
    pub const SPACE_1: f64 = 4.0;   // small gaps
    pub const SPACE_2: f64 = 8.0;   // input padding-y
    pub const SPACE_3: f64 = 12.0;  // input padding-x, message padding
    pub const SPACE_4: f64 = 16.0;  // panel padding
    pub const SPACE_05: f64 = 2.0;  // tiny gaps
    pub const SPACE_15: f64 = 6.0;  // small rounded

    // Border radius from windsurf.css
    pub const ROUNDED: f64 = 3.0;      // small elements
    pub const ROUNDED_MD: f64 = 6.0;   // inputs, messages, buttons
    pub const ROUNDED_LG: f64 = 8.0;   // panels
    pub const ROUNDED_XL: f64 = 12.0;  // large panels
    pub const ROUNDED_2XL: f64 = 16.0; // extra large
    pub const ROUNDED_PANEL: f64 = 15.0;

    // Component heights
    pub const BUTTON_HEIGHT: f64 = 28.0;      // standard button
    pub const INPUT_HEIGHT_MIN: f64 = 36.0;   // minimum input height
    pub const AVATAR_SIZE: f64 = 24.0;        // user/AI avatar
    pub const HEADER_HEIGHT: f64 = 40.0;      // panel header
}

/// Font sizes matching Windsurf (Tailwind)
pub mod font_size {
    pub const XS: f32 = 12.0;   // text-xs
    pub const SM: f32 = 13.0;   // text-sm (default)
    pub const BASE: f32 = 14.0; // text-base
    pub const LG: f32 = 16.0;   // text-lg
}
