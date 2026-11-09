//! IR → Rust that reads as hand-written Bevy code.
//!
//! Each script becomes a component (its variables and state as named fields), a plugin, and one
//! system per event. Pure nodes are inlined from their source in `vs_nodes` with the arguments
//! substituted, nodes whose inputs are all literals are folded into one literal by calling the
//! same function, and effects are written out as the transform math `vs_nodes::fx` does. So the
//! output depends on nothing but Bevy, and computes exactly what the Lua preview computes.
//!
//! Two layouts: [`emit_dev`] writes one file for a game's `build.rs` (in preview mode only the
//! components, driven by Lua), and [`emit_export`] writes `src/scripts/` for an exported project.

use std::collections::{BTreeSet, HashMap};

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::visit_mut::VisitMut;

use crate::access::{self, Filter, Resource, Subject};
use crate::ir::{Action, Declared, Device, EffectOp, Event, Intrinsic, Let, Lit, Module, Operand, Step};
use crate::library::PinType;
use crate::lower::snake_case;

/// One script: the asset path its `.lua` file has (`scripts/spinner.lua`) and its IR.
pub struct Script<'a> {
    pub path: String,
    pub ir: &'a Module,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Components only. `vs_bevy` drives them from the `.lua` files, with live reload.
    Preview,
    /// Components and their systems: the scripts as Rust.
    Native,
}

/// The Rust names a script gets, from its file name: `spinner.lua` → `spinner`, `Spinner`,
/// `SpinnerPlugin`.
#[derive(Clone, Debug)]
pub struct Names {
    pub module: String,
    pub ty: String,
    pub plugin: String,
}

/// Names that `bevy::prelude::*` already uses. A script named like one gets a suffix, so the
/// generated component doesn't shadow Bevy's type.
const PRELUDE: &[&str] = &[
    "Transform", "Time", "Name", "Camera", "Camera2d", "Camera3d", "Text", "Node", "Image", "Mesh", "Window",
    "Timer", "Sprite", "Color", "Visibility", "Children", "Parent", "Entity", "World", "App", "Plugin",
    "Commands", "Query", "Res", "Component", "Resource", "Light", "Button", "Interaction", "Scene", "Audio",
];

pub fn names(path: &str) -> Names {
    let stem = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = stem.strip_suffix(".lua").unwrap_or(stem);
    let module = ident_name(stem);
    let mut ty: String = module
        .split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default()
        })
        .collect();
    if ty.is_empty() || ty.starts_with(|c: char| c.is_ascii_digit()) {
        ty.insert_str(0, "Script");
    }
    if PRELUDE.contains(&ty.as_str()) {
        ty.push_str("Behavior");
    }
    let plugin = format!("{ty}Plugin");
    Names { module, ty, plugin }
}

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become",
    "box", "do", "final", "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Whether a declared component may not be named `name`: Bevy's prelude has it, or the
/// generated code does (`ScriptsPlugin`).
pub(crate) fn is_reserved_type(name: &str) -> bool {
    PRELUDE.contains(&name) || name == "ScriptsPlugin"
}

/// Whether `name` can be a declared component's field as written: a snake_case identifier that
/// isn't a keyword. Fields keep their names in Rust, since the preview reaches them by name.
pub(crate) fn is_field_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !KEYWORDS.contains(&name)
}

/// A snake-case identifier for any text.
fn ident_name(text: &str) -> String {
    let mut name = snake_case(text);
    name.retain(|c| c.is_ascii_alphanumeric() || c == '_');
    if name.is_empty() {
        name = "value".into();
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert_str(0, "n_");
    }
    if KEYWORDS.contains(&name.as_str()) {
        name.push('_');
    }
    name
}

/// Hands out unique names, adding `_2`, `_3`… on repeats.
#[derive(Default, Clone)]
pub(crate) struct Namer(BTreeSet<String>);

impl Namer {
    pub(crate) fn take(&mut self, base: &str) -> String {
        let base = ident_name(base);
        let mut name = base.clone();
        let mut n = 2;
        while !self.0.insert(name.clone()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        name
    }
}

fn format_file(tokens: TokenStream) -> String {
    let file: syn::File = syn::parse2(tokens).expect("generated code parses");
    prettyplease::unparse(&file)
}

/// One file for `include!` from a game's `build.rs`: a module per script, `pub use` of each
/// component, and a `ScriptsPlugin` that sets them all up.
pub fn emit_dev(scripts: &[Script], mode: Mode) -> String {
    let modules = scripts.iter().map(|s| {
        let names = names(&s.path);
        let m = format_ident!("{}", names.module);
        let items = script_items(s, &names, mode == Mode::Native);
        quote! {
            #[allow(dead_code, unused_imports)]
            pub mod #m { #items }
        }
    });
    let reexports = reexports(scripts);
    let build = match mode {
        Mode::Native => plugin_body(scripts),
        Mode::Preview => {
            // Each script's Lua runs on the entities with its component. The Lua reaches declared
            // components through reflection, by the exact type registered here.
            let registrations = scripts.iter().map(|s| {
                let names = names(&s.path);
                let (m, ty, path) = (format_ident!("{}", names.module), format_ident!("{}", names.ty), &s.path);
                let own = (!s.ir.declares_only()).then(|| quote! { ::vs_bevy::preview::register::<#m::#ty>(app, #path); });
                let declared = s.ir.declares.iter().map(|d| {
                    let (t, name) = (format_ident!("{}", d.name), &d.name);
                    quote! { ::vs_bevy::preview::declare::<#m::#t>(app, #name); }
                });
                quote! { #own #(#declared)* }
            });
            quote! {
                fn build(&self, app: &mut App) {
                    if !app.is_plugin_added::<::vs_bevy::VisualScriptPlugin>() {
                        app.add_plugins(::vs_bevy::VisualScriptPlugin);
                    }
                    #(#registrations)*
                    ::vs_bevy::preview::dump_components(app, env!("CARGO_PKG_NAME"), env!("CARGO_MANIFEST_DIR"));
                }
            }
        }
    };
    let tokens = quote! {
        use bevy::prelude::*;
        #(#modules)*
        #reexports
        pub struct ScriptsPlugin;
        impl Plugin for ScriptsPlugin { #build }
    };
    format!("// @generated from assets/scripts. Don't edit: it's rewritten on every build.\n\n{}", format_file(tokens))
}

/// The files of `src/scripts/` for an exported project, as (path relative to `src/`, source).
/// They read as hand-written code and depend only on Bevy.
pub fn emit_export(scripts: &[Script]) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mods = scripts.iter().map(|s| format_ident!("{}", names(&s.path).module));
    let reexports = reexports(scripts);
    let build = plugin_body(scripts);
    files.push((
        "scripts/mod.rs".to_string(),
        format_file(quote! {
            use bevy::prelude::*;
            #(mod #mods;)*
            #reexports
            pub struct ScriptsPlugin;
            impl Plugin for ScriptsPlugin { #build }
        }),
    ));
    for s in scripts {
        let names = names(&s.path);
        files.push((format!("scripts/{}.rs", names.module), format_file(script_items(s, &names, true))));
    }
    files
}

fn reexports(scripts: &[Script]) -> TokenStream {
    // Declared components are reached from other script modules as `super::Name`.
    let items = scripts.iter().map(|s| {
        let names = names(&s.path);
        let (m, ty) = (format_ident!("{}", names.module), format_ident!("{}", names.ty));
        let own = (!s.ir.declares_only()).then(|| quote! { pub use #m::#ty; });
        let declared = s.ir.declares.iter().map(|d| {
            let t = format_ident!("{}", d.name);
            quote! { pub use #m::#t; }
        });
        quote! { #own #(#declared)* }
    });
    quote! { #(#items)* }
}

fn plugin_body(scripts: &[Script]) -> TokenStream {
    let plugins: Vec<TokenStream> = scripts
        .iter()
        .map(|s| {
            let names = names(&s.path);
            let (m, p) = (format_ident!("{}", names.module), format_ident!("{}", names.plugin));
            quote!(#m::#p)
        })
        .collect();
    match plugins.len() {
        0 => quote! { fn build(&self, _app: &mut App) {} },
        1 => quote! { fn build(&self, app: &mut App) { app.add_plugins(#(#plugins)*); } },
        _ => quote! { fn build(&self, app: &mut App) { app.add_plugins((#(#plugins),*)); } },
    }
}

/// The script's own component and its `Default`: its variables, then its state slots. It
/// requires the components the script uses, so an entity given it always has them, in the
/// preview and in the game alike.
fn component(ir: &Module, ty: &syn::Ident, var_fields: &[String], state_fields: &[String]) -> TokenStream {
    let required: Vec<syn::Path> =
        ir.components.iter().map(|c| syn::parse_str(&c.rust_path).expect("component paths parse")).collect();
    let require = (!required.is_empty()).then(|| quote! { #[require(#(#required),*)] });
    let vars = ir.variables.iter().zip(var_fields).map(|(v, f)| (format_ident!("{f}"), v.initial.clone()));
    let states = state_fields.iter().map(|f| (format_ident!("{f}"), Lit::Bool(false)));
    let fields: Vec<(syn::Ident, Lit)> = vars.chain(states).collect();
    data_struct(ty, require, &fields, false)
}

/// A component a Define Component card declares. It's reflected, so the preview reaches its
/// fields by name and the game's catalog and inspectors see it.
fn declared(d: &Declared) -> TokenStream {
    let ty = format_ident!("{}", d.name);
    let fields: Vec<(syn::Ident, Lit)> = d.fields.iter().map(|f| (format_ident!("{}", f.name), f.initial.clone())).collect();
    data_struct(&ty, None, &fields, true)
}

/// A component struct with the given fields, and a `Default` that starts each at its literal.
fn data_struct(ty: &syn::Ident, require: Option<TokenStream>, fields: &[(syn::Ident, Lit)], reflect: bool) -> TokenStream {
    let decls = fields.iter().map(|(f, lit)| {
        let t = match lit {
            Lit::Number(_) => quote!(f32),
            Lit::Bool(_) => quote!(bool),
            Lit::Text(_) => quote!(String),
            Lit::Vec3(_) => quote!(Vec3),
            Lit::Quat(_) => quote!(Quat),
            Lit::Color(_) => quote!(Color),
        };
        quote! { pub #f: #t, }
    });
    // `Color::default()` is linear white, which no sRGB literal equals, so a color always gets
    // its own initializer.
    let all_default = fields.iter().all(|(_, lit)| match lit {
        Lit::Number(n) => n.to_bits() == 0,
        Lit::Bool(b) => !b,
        Lit::Text(s) => s.is_empty(),
        Lit::Vec3(v) | Lit::Quat(v) => v.iter().all(|n| n.to_bits() == 0),
        Lit::Color(_) => false,
    });
    let (reflect_derive, reflect_attr) = if reflect {
        (quote!(Reflect,), Some(quote! { #[reflect(Component, Default)] }))
    } else {
        (quote!(), None)
    };
    if all_default {
        return quote! {
            #[derive(Component, #reflect_derive Clone, Debug, Default)]
            #reflect_attr
            #require
            pub struct #ty { #(#decls)* }
        };
    }
    let inits = fields.iter().map(|(f, lit)| {
        let value = match lit {
            Lit::Text(s) => quote!(#s.to_string()),
            other => {
                let e = lit_expr(other, false);
                quote!(#e)
            }
        };
        quote! { #f: #value, }
    });
    quote! {
        #[derive(Component, #reflect_derive Clone, Debug)]
        #reflect_attr
        #require
        pub struct #ty { #(#decls)* }

        impl Default for #ty {
            fn default() -> Self {
                Self { #(#inits)* }
            }
        }
    }
}

/// The items of one script's module: the component, and with `systems` its plugin and one
/// system per event.
fn script_items(script: &Script, names: &Names, systems: bool) -> TokenStream {
    let ir = script.ir;
    let ty = format_ident!("{}", names.ty);
    let mut fields = Namer::default();
    let var_fields: Vec<String> = ir.variables.iter().map(|v| fields.take(&v.name)).collect();
    let state_fields: Vec<String> = ir.states.iter().map(|s| fields.take(s)).collect();
    // A script that only declares components has none of its own.
    let own = (!ir.declares_only()).then(|| component(ir, &ty, &var_fields, &state_fields));
    let declared: Vec<TokenStream> = ir.declares.iter().map(declared).collect();
    let component = quote! { #own #(#declared)* };
    if !systems {
        return quote! {
            use bevy::prelude::*;
            #component
        };
    }

    let mut uses = HashMap::new();
    for block in &ir.blocks {
        count_uses(&block.body, &mut uses);
    }
    // Each component's loop variable: `transform`, or the type in snake case (`point_light`).
    let mut bindings = Namer::default();
    for taken in [names.module.as_str(), "transform", "time", "keys", "mouse", "commands", "entity", "query"] {
        bindings.take(taken);
    }
    let component_bindings: Vec<syn::Ident> = ir
        .components
        .iter()
        .map(|c| {
            if c.name == "Transform" {
                format_ident!("transform")
            } else {
                format_ident!("{}", bindings.take(&c.name))
            }
        })
        .collect();
    let cx = Cx {
        ir,
        binding: format_ident!("{}", names.module),
        var_fields: &var_fields,
        state_fields: &state_fields,
        uses: &uses,
        component_bindings: &component_bindings,
        reserved: bindings,
    };
    // Start events first, then the rest in graph order, as both runtimes run them.
    let mut system_fns = Vec::new();
    let mut schedule = Vec::new();
    let mut needs_condition = false;
    for system in access::systems(ir, names) {
        let block = &ir.blocks[system.block];
        let fn_name = format_ident!("{}", system.name);
        system_fns.push(cx.system(&fn_name, &system, &block.body));
        schedule.push(match &block.event {
            Event::Key(key) => {
                needs_condition = true;
                let key = format_ident!("{key}");
                quote!(#fn_name.run_if(input_just_pressed(KeyCode::#key)))
            }
            _ => quote!(#fn_name),
        });
    }
    let plugin = format_ident!("{}", names.plugin);
    // Declared components are registered, so reflection (the catalog, inspectors) finds them.
    let registrations: Vec<TokenStream> = ir
        .declares
        .iter()
        .map(|d| {
            let t = format_ident!("{}", d.name);
            quote! { app.register_type::<#t>(); }
        })
        .collect();
    let add_systems = match schedule.len() {
        0 => None,
        1 => Some(quote! { app.add_systems(Update, #(#schedule)*); }),
        _ => Some(quote! { app.add_systems(Update, (#(#schedule),*).chain()); }),
    };
    let build = if registrations.is_empty() && add_systems.is_none() {
        quote! { fn build(&self, _app: &mut App) {} }
    } else {
        quote! { fn build(&self, app: &mut App) { #(#registrations)* #add_systems } }
    };
    let condition_use = needs_condition.then(|| quote! { use bevy::input::common_conditions::input_just_pressed; });
    quote! {
        #condition_use
        use bevy::prelude::*;

        #component

        pub struct #plugin;

        impl Plugin for #plugin { #build }

        #(#system_fns)*
    }
}

fn count_uses(steps: &[Step], uses: &mut HashMap<u32, usize>) {
    fn add(op: &Operand, uses: &mut HashMap<u32, usize>) {
        if let Operand::Local(n) = op {
            *uses.entry(*n).or_default() += 1;
        }
    }
    for step in steps {
        for arg in step.lets.iter().flat_map(|l| &l.args) {
            add(arg, uses);
        }
        match &step.action {
            Action::If { cond, then, otherwise } => {
                add(cond, uses);
                count_uses(then, uses);
                count_uses(otherwise, uses);
            }
            Action::SetVar { value, .. } | Action::SetState { value, .. } | Action::SetField { value, .. } => {
                add(value, uses)
            }
            Action::Effect { args, .. } => args.iter().for_each(|a| add(a, uses)),
            Action::Despawn => {}
        }
    }
}

/// What a local stands for in the code being written.
#[derive(Clone)]
enum Local {
    /// A value known at generation time (every input was a literal).
    Lit(Lit),
    /// An expression pasted in where the local is used once.
    Inline(syn::Expr),
    /// A `let` binding.
    Bound(syn::Ident),
}

type Env = HashMap<u32, Local>;

struct Cx<'a> {
    ir: &'a Module,
    /// The loop variable holding the component, named after the script (`spinner`).
    binding: syn::Ident,
    var_fields: &'a [String],
    state_fields: &'a [String],
    uses: &'a HashMap<u32, usize>,
    /// The loop variable of each component in `Module::components`.
    component_bindings: &'a [syn::Ident],
    /// Names already taken by loop variables, so locals don't shadow them.
    reserved: Namer,
}

impl Cx<'_> {
    /// One system: a query over the entities with the component, asking only for what the
    /// event's steps use (see [`access`]), and the steps for each entity.
    fn system(&self, name: &syn::Ident, system: &access::System, steps: &[Step]) -> TokenStream {
        let mut names = self.reserved.clone();
        let body = self.steps(steps, &mut Env::new(), &mut names);
        let ty = format_ident!("{}", system.own);

        let mut items = Vec::new();
        let mut patterns = Vec::new();
        for item in &system.items {
            let bind = match item.subject {
                Subject::Entity => {
                    items.push(quote!(Entity));
                    patterns.push(quote!(entity));
                    continue;
                }
                Subject::Transform => format_ident!("transform"),
                Subject::Own => self.binding.clone(),
                Subject::Component(i) => self.component_bindings[i].clone(),
            };
            let path: syn::Path = syn::parse_str(&item.rust_path).expect("component paths parse");
            if item.write {
                items.push(quote!(&mut #path));
                patterns.push(quote!(mut #bind));
            } else {
                items.push(quote!(&#path));
                patterns.push(quote!(#bind));
            }
        }
        let (item, pattern) = match items.len() {
            0 => (quote!(()), quote!(())),
            1 => (items.remove(0), patterns.remove(0)),
            _ => (quote!((#(#items),*)), quote!((#(#patterns),*))),
        };
        let query = match system.filter {
            Some(Filter::Added) => quote!(Query<#item, Added<#ty>>),
            Some(Filter::With) => quote!(Query<#item, With<#ty>>),
            None => quote!(Query<#item>),
        };
        let mutable = system.mutable();

        let mut params = Vec::new();
        for resource in &system.resources {
            params.push(match resource {
                Resource::Time => quote!(time: Res<Time>),
                Resource::Keys => quote!(keys: Res<ButtonInput<KeyCode>>),
                Resource::Mouse => quote!(mouse: Res<ButtonInput<MouseButton>>),
            });
        }
        if system.commands {
            params.push(quote!(mut commands: Commands));
        }
        if mutable {
            params.push(quote!(mut query: #query));
        } else {
            params.push(quote!(query: #query));
        }
        let iter = if mutable { quote!(&mut query) } else { quote!(&query) };
        quote! {
            fn #name(#(#params),*) {
                for #pattern in #iter {
                    #body
                }
            }
        }
    }

    /// `point_light.intensity`, `text_color.0`.
    fn field(&self, component: usize, path: &str) -> syn::Expr {
        let bind = &self.component_bindings[component];
        syn::parse_str(&format!("{bind}.{path}")).expect("field paths parse")
    }

    fn steps(&self, steps: &[Step], env: &mut Env, names: &mut Namer) -> TokenStream {
        let mut out = TokenStream::new();
        for step in steps {
            for l in &step.lets {
                if let Some(binding) = self.bind(l, env, names) {
                    out.extend(binding);
                }
            }
            out.extend(self.action(&step.action, env, names));
        }
        out
    }

    /// Records what a `Let` stands for. Returns a `let` statement when the value needs one: it's
    /// used more than once, or the user named its node.
    fn bind(&self, l: &Let, env: &mut Env, names: &mut Namer) -> Option<TokenStream> {
        let literal_args: Option<Vec<vs_nodes::Arg>> = l
            .args
            .iter()
            .map(|a| match a {
                Operand::Lit(lit) => Some(to_arg(lit)),
                Operand::Local(n) => match env.get(n) {
                    Some(Local::Lit(lit)) => Some(to_arg(lit)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if let Some(value) = literal_args.and_then(|args| vs_nodes::call(l.node, &args)).and_then(from_arg) {
            env.insert(l.local, Local::Lit(value));
            return None;
        }
        let args: Vec<syn::Expr> = l.args.iter().map(|a| self.operand(a, env, true)).collect();
        let expr = inline(l.node, args);
        let uses = self.uses.get(&l.local).copied().unwrap_or(0);
        if uses == 0 {
            return None;
        }
        if uses == 1 && !l.named {
            env.insert(l.local, Local::Inline(expr));
            return None;
        }
        let name = format_ident!("{}", names.take(&l.name));
        env.insert(l.local, Local::Bound(name.clone()));
        Some(quote! { let #name = #expr; })
    }

    fn action(&self, action: &Action, env: &mut Env, names: &mut Namer) -> TokenStream {
        let b = &self.binding;
        match action {
            Action::If { cond, then, otherwise } => {
                let cond = self.operand(cond, env, false);
                let then = self.steps(then, &mut env.clone(), names);
                if otherwise.is_empty() {
                    quote! { if #cond { #then } }
                } else {
                    let otherwise = self.steps(otherwise, &mut env.clone(), names);
                    quote! { if #cond { #then } else { #otherwise } }
                }
            }
            Action::SetVar { var, value } => {
                let f = format_ident!("{}", self.var_fields[*var]);
                assign(quote! { #b.#f }, self.operand(value, env, true))
            }
            Action::SetState { slot, value } => {
                let f = format_ident!("{}", self.state_fields[*slot]);
                assign(quote! { #b.#f }, self.operand(value, env, true))
            }
            Action::Despawn => quote! { commands.entity(entity).despawn(); },
            Action::SetField { component, path, value } => {
                let target = self.field(*component, path);
                assign(quote!(#target), self.operand(value, env, true))
            }
            Action::Effect { op: EffectOp::Print, args } => {
                let literal = match &args[0] {
                    Operand::Lit(Lit::Text(s)) => Some(s.clone()),
                    Operand::Local(n) => match env.get(n) {
                        Some(Local::Lit(Lit::Text(s))) => Some(s.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                match literal {
                    Some(s) => {
                        let s = s.replace('{', "{{").replace('}', "}}");
                        quote! { info!(#s); }
                    }
                    None => {
                        let text = self.operand(&args[0], env, false);
                        // Macro arguments aren't formatted, so anything but a plain field or
                        // name gets its own `let`.
                        if matches!(text, syn::Expr::Path(_) | syn::Expr::Field(_)) {
                            quote! { info!("{}", #text); }
                        } else {
                            let m = format_ident!("{}", names.take("message"));
                            quote! {
                                let #m = #text;
                                info!("{}", #m);
                            }
                        }
                    }
                }
            }
            Action::Effect { op, args } => {
                let values: Vec<syn::Expr> = args.iter().map(|a| self.operand(a, env, true)).collect();
                let reads_transform = values.iter().any(|v| quote!(#v).to_string().contains("transform"));
                match op {
                    EffectOp::Rotate => {
                        // fx::rotate: Euler angles in degrees, XYZ order, applied in local space,
                        // then normalized.
                        let radians = values.iter().map(|v| match lit_value(v) {
                            Some(n) if n == 0.0 => v.clone(),
                            Some(_) => method(suffixed(v), "to_radians"),
                            None => method(v.clone(), "to_radians"),
                        });
                        let r = format_ident!("{}", names.take("rotation"));
                        quote! {
                            let #r = Quat::from_euler(EulerRot::XYZ, #(#radians),*);
                            transform.rotation = (transform.rotation * #r).normalize();
                        }
                    }
                    EffectOp::Translate if reads_transform => {
                        let o = format_ident!("{}", names.take("offset"));
                        quote! {
                            let #o = Vec3::new(#(#values),*);
                            transform.translation += #o;
                        }
                    }
                    EffectOp::Translate => {
                        let v = vec3(&values);
                        quote! { transform.translation += #v; }
                    }
                    EffectOp::SetPosition => {
                        let v = vec3(&values);
                        quote! { transform.translation = #v; }
                    }
                    EffectOp::SetScale => {
                        let v = vec3(&values);
                        quote! { transform.scale = #v; }
                    }
                    EffectOp::Print => unreachable!("handled above"),
                }
            }
        }
    }

    /// The expression for an operand. `owned` asks for a value that can be moved, so text
    /// fields are cloned.
    fn operand(&self, op: &Operand, env: &Env, owned: bool) -> syn::Expr {
        let b = &self.binding;
        match op {
            Operand::Lit(lit) => lit_expr(lit, false),
            Operand::Local(n) => match env.get(n) {
                Some(Local::Lit(lit)) => lit_expr(lit, false),
                Some(Local::Inline(e)) => e.clone(),
                Some(Local::Bound(id)) => syn::parse_quote!(#id),
                None => unreachable!("locals are bound before use"),
            },
            Operand::Var(i) => {
                let f = format_ident!("{}", self.var_fields[*i]);
                if owned && matches!(self.ir.variables[*i].initial, Lit::Text(_)) {
                    syn::parse_quote!(#b.#f.clone())
                } else {
                    syn::parse_quote!(#b.#f)
                }
            }
            Operand::State(i) => {
                let f = format_ident!("{}", self.state_fields[*i]);
                syn::parse_quote!(#b.#f)
            }
            Operand::Field { component, path, ty } => {
                let field = self.field(*component, path);
                if owned && *ty == PinType::String { syn::parse_quote!(#field.clone()) } else { field }
            }
            Operand::Intrinsic(Intrinsic::Delta) => syn::parse_quote!(time.delta_secs()),
            Operand::Intrinsic(Intrinsic::Time) => syn::parse_quote!(time.elapsed_secs()),
            Operand::Intrinsic(Intrinsic::Button { device, press, name }) => {
                let name = format_ident!("{name}");
                let method = format_ident!("{}", press.method());
                match device {
                    Device::Key => syn::parse_quote!(keys.#method(KeyCode::#name)),
                    Device::Mouse => syn::parse_quote!(mouse.#method(MouseButton::#name)),
                }
            }
            Operand::Intrinsic(Intrinsic::Position(axis)) => {
                let axis = format_ident!("{}", axis.name());
                syn::parse_quote!(transform.translation.#axis)
            }
        }
    }
}

/// `Vec3::new(x, y, z)`, or `Vec3::ONE` / `Vec3::ZERO` / `Vec3::splat(v)` when all three are the
/// same literal. All of them build the same bits.
fn vec3(values: &[syn::Expr]) -> TokenStream {
    let lits: Vec<Option<f32>> = values.iter().map(lit_value).collect();
    if let [Some(a), Some(b), Some(c)] = lits[..]
        && a.to_bits() == b.to_bits()
        && b.to_bits() == c.to_bits()
    {
        if a.to_bits() == 1.0f32.to_bits() {
            return quote!(Vec3::ONE);
        }
        if a.to_bits() == 0 {
            return quote!(Vec3::ZERO);
        }
        let v = &values[0];
        return quote!(Vec3::splat(#v));
    }
    quote!(Vec3::new(#(#values),*))
}

fn to_arg(lit: &Lit) -> vs_nodes::Arg {
    match lit {
        Lit::Number(n) => vs_nodes::Arg::Number(*n),
        Lit::Bool(b) => vs_nodes::Arg::Bool(*b),
        Lit::Text(s) => vs_nodes::Arg::Text(s.clone()),
        Lit::Vec3(v) => vs_nodes::Arg::Vec3(bevy_math_vec3(*v)),
        Lit::Quat([x, y, z]) => vs_nodes::Arg::Quat(vs_nodes::fx::euler(*x, *y, *z)),
        Lit::Color([r, g, b, a]) => vs_nodes::Arg::Color(vs_nodes::fx::srgba(*r, *g, *b, *a)),
    }
}

fn bevy_math_vec3([x, y, z]: [f32; 3]) -> vs_nodes::fx::Vec3 {
    vs_nodes::fx::Vec3::new(x, y, z)
}

/// A folded result as a literal. Rotations and colors aren't folded: a computed rotation has no
/// Euler form to write, and a computed color may not be sRGB, so the call stays in the code.
fn from_arg(arg: vs_nodes::Arg) -> Option<Lit> {
    match arg {
        vs_nodes::Arg::Number(n) => Some(Lit::Number(n)),
        vs_nodes::Arg::Bool(b) => Some(Lit::Bool(b)),
        vs_nodes::Arg::Text(s) => Some(Lit::Text(s)),
        vs_nodes::Arg::Vec3(v) => Some(Lit::Vec3(v.to_array())),
        vs_nodes::Arg::Quat(_) | vs_nodes::Arg::Color(_) => None,
    }
}

/// A literal as an expression. Numbers are written so they parse back to the exact `f32`
/// (`90.0`, `0.25`, `-3.0`); `suffix` adds `_f32`, for literals a method is called on.
fn lit_expr(lit: &Lit, suffix: bool) -> syn::Expr {
    let numbers = |v: &[f32]| -> Vec<syn::Expr> { v.iter().map(|n| lit_expr(&Lit::Number(*n), false)).collect() };
    match lit {
        Lit::Bool(b) => syn::parse_quote!(#b),
        Lit::Text(s) => syn::parse_quote!(String::from(#s)),
        Lit::Vec3(v) => syn::parse2(vec3(&numbers(v))).expect("vectors parse"),
        Lit::Quat(euler) => {
            if euler.iter().all(|n| n.to_bits() == 0) {
                return syn::parse_quote!(Quat::IDENTITY);
            }
            let radians = numbers(euler).into_iter().map(|v| match lit_value(&v) {
                Some(n) if n == 0.0 => v,
                _ => method(suffixed(&v), "to_radians"),
            });
            syn::parse_quote!(Quat::from_euler(EulerRot::XYZ, #(#radians),*))
        }
        Lit::Color(rgba) => {
            let c = numbers(rgba);
            if rgba[3].to_bits() == 1.0f32.to_bits() {
                let rgb = &c[..3];
                syn::parse_quote!(Color::srgb(#(#rgb),*))
            } else {
                syn::parse_quote!(Color::srgba(#(#c),*))
            }
        }
        Lit::Number(n) => {
            let n = *n;
            if n.is_nan() {
                return syn::parse_quote!(f32::NAN);
            }
            if n.is_infinite() {
                return if n > 0.0 { syn::parse_quote!(f32::INFINITY) } else { syn::parse_quote!(f32::NEG_INFINITY) };
            }
            let mut text = format!("{:?}", n.abs());
            if !text.contains(['.', 'e']) {
                text.push_str(".0");
            }
            if suffix {
                text.push_str("_f32");
            }
            let lit = syn::LitFloat::new(&text, Span::call_site());
            if n.is_sign_negative() { syn::parse_quote!(-#lit) } else { syn::parse_quote!(#lit) }
        }
    }
}

/// The value of a number literal expression (`2.0`, `-3.0`), if it is one.
fn lit_value(e: &syn::Expr) -> Option<f32> {
    match e {
        syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Float(f), .. }) => f.base10_parse().ok(),
        syn::Expr::Unary(syn::ExprUnary { op: syn::UnOp::Neg(_), expr, .. }) => lit_value(expr).map(|v| -v),
        _ => None,
    }
}

/// `receiver.name()`, built as syntax so the printer adds parentheses where precedence needs them
/// (`(a * b).to_radians()`). Interpolating tokens would lose them.
fn method(receiver: syn::Expr, name: &str) -> syn::Expr {
    syn::Expr::MethodCall(syn::ExprMethodCall {
        attrs: Vec::new(),
        receiver: Box::new(receiver),
        dot_token: Default::default(),
        method: format_ident!("{name}"),
        turbofish: None,
        paren_token: Default::default(),
        args: Default::default(),
    })
}

/// The same literal with an `_f32` suffix, so a method can be called on it.
fn suffixed(e: &syn::Expr) -> syn::Expr {
    match lit_value(e) {
        Some(v) if v.is_finite() => lit_expr(&Lit::Number(v), true),
        _ => e.clone(),
    }
}

/// A pure node's body with its parameters replaced by `args`.
fn inline(kind: &str, args: Vec<syn::Expr>) -> syn::Expr {
    let node = vs_nodes::catalog().into_iter().find(|n| n.kind == kind).expect("lowering only emits known nodes");
    let block: syn::Block = syn::parse_str(node.body).expect("node bodies parse");
    let mut expr = match block.stmts.as_slice() {
        [syn::Stmt::Expr(e, None)] => e.clone(),
        _ => syn::Expr::Block(syn::ExprBlock { attrs: Vec::new(), label: None, block }),
    };
    let params = node.inputs.iter().map(|(name, _)| name.clone()).zip(args).collect();
    Substitute(params).visit_expr_mut(&mut expr);
    FixReceivers.visit_expr_mut(&mut expr);
    expr
}

struct Substitute(Vec<(String, syn::Expr)>);

impl VisitMut for Substitute {
    fn visit_expr_mut(&mut self, e: &mut syn::Expr) {
        if let syn::Expr::Path(p) = e
            && let Some(id) = p.path.get_ident()
            && let Some((_, arg)) = self.0.iter().find(|(name, _)| id == name)
        {
            *e = arg.clone();
            return;
        }
        syn::visit_mut::visit_expr_mut(self, e);
    }

    /// Macro arguments are tokens, not expressions. When they parse as a list of expressions (as
    /// in `format!("{:?}", value)`), substitute in those too. Captures inside the format string
    /// (`"{value}"`) can't be reached, so node bodies don't use them.
    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        use syn::parse::Parser;
        use syn::punctuated::Punctuated;
        if let Ok(mut args) = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated.parse2(m.tokens.clone()) {
            for arg in args.iter_mut() {
                self.visit_expr_mut(arg);
            }
            m.tokens = quote!(#args);
        }
    }
}

/// `2.0.min(x)` doesn't compile (the literal's type is ambiguous), so literals a method is
/// called on get a suffix: `2.0_f32.min(x)`.
struct FixReceivers;

impl VisitMut for FixReceivers {
    fn visit_expr_method_call_mut(&mut self, m: &mut syn::ExprMethodCall) {
        if lit_value(&m.receiver).is_some() {
            *m.receiver = suffixed(&m.receiver);
        }
        syn::visit_mut::visit_expr_method_call_mut(self, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_file() {
        let n = names("scripts/door_logic.lua");
        assert_eq!((n.module.as_str(), n.ty.as_str(), n.plugin.as_str()), ("door_logic", "DoorLogic", "DoorLogicPlugin"));
        assert_eq!(names("scripts/transform.lua").ty, "TransformBehavior");
        assert_eq!(names("scripts/3d.lua").module, "n_3d");
    }

    #[test]
    fn inlines_bodies_with_precedence() {
        let a: syn::Expr = syn::parse_quote!(x + y);
        let e = inline("math.mul", vec![a, lit_expr(&Lit::Number(2.0), false)]);
        assert_eq!(quote!(#e).to_string(), "(x + y) * 2.0");
        let s = inline("math.min", vec![lit_expr(&Lit::Number(2.0), false), syn::parse_quote!(x)]);
        assert!(quote!(#s).to_string().starts_with("2.0_f32 . min"), "{}", quote!(#s));
    }

    fn declare(g: &mut crate::Graph, name: &str, fields: &[(&str, crate::Value)]) {
        let fields = fields.iter().map(|(n, v)| crate::Variable { name: n.to_string(), value: v.clone() }).collect();
        g.add_component_def(crate::ComponentDef { name: name.into(), fields }, 0.0, 0.0);
    }

    #[test]
    fn a_declaring_script_gets_a_reflected_registered_component_and_none_of_its_own() {
        let mut g = crate::Graph::default();
        declare(&mut g, "Health", &[("max", crate::Value::Number(100.0)), ("alive", crate::Value::Bool(true))]);
        let lowered = crate::lower(&g);
        assert!(lowered.problems.is_empty(), "{:?}", lowered.problems);
        assert!(lowered.module.declares_only());
        let script = || Script { path: "scripts/health.lua".into(), ir: &lowered.module };

        let native = emit_dev(&[script()], Mode::Native);
        assert!(native.contains("#[derive(Component, Reflect, Clone, Debug)]"), "{native}");
        assert!(native.contains("#[reflect(Component, Default)]"), "{native}");
        assert!(native.contains("max: 100.0"), "{native}");
        assert!(native.contains("app.register_type::<Health>();"), "{native}");
        assert!(native.contains("pub use health::Health;"), "{native}");
        // Its own component would be `Health` too.
        assert_eq!(native.matches("pub struct Health ").count(), 1, "{native}");

        let preview = emit_dev(&[script()], Mode::Preview);
        assert!(preview.contains("::vs_bevy::preview::declare::<health::Health>(app, \"Health\");"), "{preview}");
        assert!(!preview.contains("preview::register"), "{preview}");
    }

    #[test]
    fn a_script_requires_the_components_it_declares_and_uses() {
        let mut g = crate::Graph::default();
        declare(&mut g, "Charge", &[("level", crate::Value::Number(0.0))]);
        let update = g.add_node("event.update", None, 0.0, 0.0);
        let set = g.add_field_node("component.set", "Charge.level", 0.0, 0.0);
        assert!(g.link(crate::PinRef::new(update, "then"), crate::PinRef::new(set, "in")));
        let lowered = crate::lower(&g);
        assert!(lowered.problems.is_empty(), "{:?}", lowered.problems);
        assert!(!lowered.module.declares_only());
        let native = emit_dev(&[Script { path: "scripts/battery.lua".into(), ir: &lowered.module }], Mode::Native);
        assert!(native.contains("#[require(super::Charge)]"), "{native}");
        assert!(native.contains("pub struct Battery"), "{native}");
        assert!(native.contains("#[derive(Component, Reflect, Clone, Debug, Default)]"), "{native}");
        assert!(native.contains("pub use battery::Battery;\npub use battery::Charge;"), "{native}");
    }

    #[test]
    fn declarations_need_free_names_and_snake_case_fields() {
        let errors = |name: &str, fields: &[&str]| {
            let mut g = crate::Graph::default();
            let fields: Vec<(&str, crate::Value)> = fields.iter().map(|f| (*f, crate::Value::Number(0.0))).collect();
            declare(&mut g, name, &fields);
            crate::lower(&g).problems.into_iter().map(|p| p.message).collect::<Vec<_>>()
        };
        assert!(errors("Health", &["max", "regen_rate", "_x2"]).is_empty());
        assert!(errors("health", &[])[0].contains("isn't a component name"));
        assert!(errors("Transform", &[])[0].contains("already a Bevy type"));
        assert!(errors("PointLight", &[])[0].contains("already a Bevy type"));
        assert!(errors("Health", &["Max"])[0].contains("snake_case"));
        assert!(errors("Health", &["type"])[0].contains("snake_case"));
        assert!(errors("Health", &["max", "max"])[0].contains("two fields named `max`"));

        let mut g = crate::Graph::default();
        declare(&mut g, "Health", &[]);
        declare(&mut g, "Health", &[]);
        let problems = crate::lower(&g).problems;
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].message.contains("declared twice"));
        assert_eq!(problems[0].node, Some(g.nodes[1].id));
    }
}

/// `target = value;`, or `target += rest;` when the value is `target + rest` (likewise `-`, `*`
/// and `/`), as a person would write it. Same operation, so the same result.
fn assign(target: TokenStream, value: syn::Expr) -> TokenStream {
    if let syn::Expr::Binary(bin) = &value {
        let (left, right) = (&bin.left, &bin.right);
        if quote!(#left).to_string() == target.to_string() {
            let op = match bin.op {
                syn::BinOp::Add(_) => Some(quote!(+=)),
                syn::BinOp::Sub(_) => Some(quote!(-=)),
                syn::BinOp::Mul(_) => Some(quote!(*=)),
                syn::BinOp::Div(_) => Some(quote!(/=)),
                _ => None,
            };
            if let Some(op) = op {
                return quote! { #target #op #right; };
            }
        }
    }
    quote! { #target = #value; }
}
