use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::{icon, subtle_surface};
use aura_core::templating::{TemplateContext, resolve_builtin_dynamic};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

const BUILTIN_DYNAMIC_DOCS: &[(&str, &str)] = &[
    ("{{$uuid}}", "Fresh UUID v4"),
    ("{{$timestamp}}", "Unix epoch seconds"),
    ("{{$isoTimestamp}}", "ISO-8601 UTC timestamp"),
    ("{{$randomInt}}", "Random integer (0..9999)"),
    ("{{$randomFloat}}", "Random float (0.00..100.00)"),
    ("{{$randomBool}}", "Random boolean"),
    ("{{$randomEmail}}", "Offline generated email"),
    ("{{$randomFullName}}", "Offline full name"),
    ("{{$faker.internet.email}}", "Faker email address"),
    ("{{$faker.person.fullName}}", "Faker full name"),
    ("{{$faker.location.city}}", "Faker city name"),
    ("{{$faker.company.name}}", "Faker company name"),
    ("{{$faker.commerce.price}}", "Faker price string"),
    ("{{$faker.lorem.sentence}}", "Faker placeholder sentence"),
];

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_variables_panel<V: 'static>(
        variable_rows: &[(String, String, bool)],
        template_ctx: &TemplateContext,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let row_active_bg = cx.theme().secondary.opacity(0.35);
        let row_disabled_bg = cx.theme().muted.opacity(0.2);
        let focus_border = cx.theme().primary;
        let field_bg = cx.theme().background;
        let surface = subtle_surface(cx);
        let radius = cx.theme().radius;

        let mut env_vars: Vec<(String, String)> = template_ctx
            .environment_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        env_vars.sort_by(|a, b| a.0.cmp(&b.0));

        let mut col_vars: Vec<(String, String)> = template_ctx
            .collection_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        col_vars.sort_by(|a, b| a.0.cmp(&b.0));

        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("REQUEST VARIABLES"),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{}", variable_rows.len())),
                            ),
                    )
                    .child(
                        Button::new("add-variable-btn")
                            .icon(icon(IconName::Plus))
                            .label("Add Variable")
                            .primary()
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::AddVariableDirect, window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("Cascade precedence: Request → Active Environment → Collection → Built-in ({{$...}}) → OS Env. Secret URIs: vault://, aws-sm://, keychain://."),
            )
            .when(variable_rows.is_empty(), |el| {
                el.child(
                    Empty::new()
                        .header(EmptyHeader::new().description(
                            EmptyDescription::new().child(
                                "No request variables defined. Click 'Add Variable' to add one.",
                            ),
                        ))
                        .child(
                            Button::new("btn-add-first-variable")
                                .icon(icon(IconName::Plus))
                                .label("Add First Variable")
                                .secondary()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::AddVariableDirect,
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
            })
            .children(variable_rows.iter().enumerate().map(|(idx, (key, val, enabled))| {
                let enabled = *enabled;
                let is_key_focused =
                    matches!(active_input, Some(ActiveInputTarget::VariableKey(i)) if *i == idx);
                let is_val_focused =
                    matches!(active_input, Some(ActiveInputTarget::VariableValue(i)) if *i == idx);
                let view_entity = cx.entity().clone();

                h_flex()
                    .id(ElementId::NamedInteger("variable-row".into(), idx as u64))
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded(radius)
                    .bg(if enabled {
                        row_active_bg
                    } else {
                        row_disabled_bg
                    })
                    .border_1()
                    .border_color(if is_key_focused || is_val_focused {
                        focus_border
                    } else {
                        border
                    })
                    .child(
                        Checkbox::new(ElementId::NamedInteger("toggle-var".into(), idx as u64))
                            .checked(enabled)
                            .small()
                            .on_click(move |_, window, app| {
                                let _ = view_entity.update(app, |this, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::ToggleVariable(idx),
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    )
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-var-key".into(), idx as u64),
                        inputs.get_input(&InputKey::VariableKey(idx)),
                        key,
                        is_key_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(div().text_xs().text_color(muted).child("="))
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-var-val".into(), idx as u64),
                        inputs.get_input(&InputKey::VariableValue(idx)),
                        val,
                        is_val_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(
                        Button::new(ElementId::NamedInteger("remove-var".into(), idx as u64))
                            .icon(icon(IconName::X))
                            .xsmall()
                            .ghost()
                            .tooltip("Remove variable")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::RemoveVariable(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
            }))
            .child(Separator::horizontal().my_1())
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded(radius)
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(normal)
                                    .child("ACTIVE ENVIRONMENT & COLLECTION VARIABLES"),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{} env · {} col", env_vars.len(), col_vars.len())),
                            ),
                    )
                    .child(if env_vars.is_empty() && col_vars.is_empty() {
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("No active environment or collection variables in scope.")
                            .into_any_element()
                    } else {
                        let mut list = v_flex().gap_1();
                        for (scope, items) in [("ENV", env_vars), ("COL", col_vars)] {
                            for (k, v) in items {
                                list = list.child(
                                    h_flex()
                                        .justify_between()
                                        .gap_2()
                                        .px_2()
                                        .py_1()
                                        .rounded(radius)
                                        .bg(field_bg)
                                        .border_1()
                                        .border_color(border)
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(Tag::info().xsmall().child(scope))
                                                .child(
                                                    div()
                                                        .font_family("monospace")
                                                        .text_xs()
                                                        .text_color(normal)
                                                        .child(format!("{{{{{k}}}}}")),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .font_family("monospace")
                                                .text_xs()
                                                .text_color(muted)
                                                .truncate()
                                                .child(v),
                                        ),
                                );
                            }
                        }
                        list.into_any_element()
                    }),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded(radius)
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(normal)
                                    .child("BUILT-IN DYNAMIC & FAKER VARIABLES"),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{} built-ins", BUILTIN_DYNAMIC_DOCS.len())),
                            ),
                    )
                    .child({
                        let mut grid = v_flex().gap_1();
                        for (token, desc) in BUILTIN_DYNAMIC_DOCS {
                            let inner = token.trim_start_matches("{{").trim_end_matches("}}");
                            let sample = resolve_builtin_dynamic(inner).unwrap_or_default();
                            grid = grid.child(
                                h_flex()
                                    .justify_between()
                                    .gap_2()
                                    .px_2()
                                    .py_1()
                                    .rounded(radius)
                                    .bg(field_bg)
                                    .border_1()
                                    .border_color(border)
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(focus_border)
                                                    .child(*token),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(muted)
                                                    .child(*desc),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .font_family("monospace")
                                            .text_xs()
                                            .text_color(muted)
                                            .max_w(px(220.))
                                            .truncate()
                                            .child(format!("e.g. {sample}")),
                                    ),
                            );
                        }
                        grid
                    }),
            )
    }
}
