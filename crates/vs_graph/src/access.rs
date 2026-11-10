//! What each event's system asks Bevy for: its query's data and filter, the resources it reads
//! and whether it needs `Commands`. Read from the IR alone, so the editor shows it next to each
//! event without generating code, and the Rust backend writes its systems from it, so what the
//! editor shows is what compiles.
//!
//! Input: a script's [`Module`] and its Rust [`Names`]. Output: one [`System`] per event, in the
//! order the script's plugin adds them (Start events first, then the rest in graph order).
//!
//! From several scripts' systems, [`ambiguities`] finds the pairs Bevy's ambiguity detection
//! (`ScheduleBuildSettings::ambiguity_detection`) reports, by the same rule Bevy applies.

use std::collections::BTreeSet;

use crate::FilterOp;
use crate::ir::{Action, Device, EffectOp, Event, Intrinsic, Module, Operand, Step};
use crate::lower::snake_case;
use crate::rust::{Names, Namer};

/// What a query item stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    /// The entity's id, for despawning it.
    Entity,
    Transform,
    /// The script's own component: its variables and state.
    Own,
    /// A component the script reads or writes fields of, by index into [`Module::components`].
    Component(usize),
}

/// One item of a system's query, such as `&mut Transform`.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub subject: Subject,
    /// The component's short name, as Bevy names it in its diagnostics (`PointLight`). Empty
    /// for [`Subject::Entity`].
    pub name: String,
    /// How the generated code names the type (`bevy::light::PointLight`, `super::Stats`).
    pub rust_path: String,
    pub write: bool,
}

/// The query's filter on the script's own component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// Only entities that got the component since the system last ran: a Start event. Bevy
    /// counts it as reading the component.
    Added,
    /// Entities with the component, when the query doesn't ask for it. Bevy doesn't count it as
    /// an access.
    With,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Resource {
    Time,
    Keys,
    Mouse,
}

impl Resource {
    /// The resource's type, as written in a system's parameters.
    pub fn rust(self) -> &'static str {
        match self {
            Resource::Time => "Time",
            Resource::Keys => "ButtonInput<KeyCode>",
            Resource::Mouse => "ButtonInput<MouseButton>",
        }
    }
}

/// One event's system.
#[derive(Clone, Debug, PartialEq)]
pub struct System {
    /// The function's name: the event node's name, or `update_spinner`, `on_space_spinner`…
    pub name: String,
    /// Which block of [`Module::blocks`] it runs.
    pub block: usize,
    /// The script's own component, which the filter is on (`Spinner`).
    pub own: String,
    /// The query's data, in the order the generated code asks for it.
    pub items: Vec<Item>,
    pub filter: Option<Filter>,
    /// The filters the event node adds, after the one on the own component.
    pub filters: Vec<crate::ir::Filter>,
    /// Resources it reads. None are written.
    pub resources: BTreeSet<Resource>,
    pub commands: bool,
}

impl System {
    /// Whether any query item is mutable, so the query is declared `mut`.
    pub fn mutable(&self) -> bool {
        self.items.iter().any(|i| i.write)
    }

    /// The query as a Rust type with short names: `Query<(&mut Transform, &Spinner)>`.
    pub fn query(&self) -> String {
        match self.filter_type() {
            Some(filters) => format!("Query<{}, {filters}>", self.data()),
            None => format!("Query<{}>", self.data()),
        }
    }

    /// The query's data, its first type parameter: `(&mut Transform, &Spinner)`.
    pub fn data(&self) -> String {
        let items: Vec<String> = self
            .items
            .iter()
            .map(|i| match (i.subject, i.write) {
                (Subject::Entity, _) => "Entity".to_string(),
                (_, true) => format!("&mut {}", i.name),
                (_, false) => format!("&{}", i.name),
            })
            .collect();
        match items.len() {
            0 => "()".to_string(),
            1 => items[0].clone(),
            _ => format!("({})", items.join(", ")),
        }
    }

    /// The query's filters, its second type parameter, if it has any: `With<Spinner>`, or
    /// `(With<Spinner>, Without<Enemy>)`.
    pub fn filter_type(&self) -> Option<String> {
        let filters = self.filter_names();
        match filters.len() {
            0 => None,
            1 => Some(filters[0].clone()),
            _ => Some(format!("({})", filters.join(", "))),
        }
    }

    /// The query's filters with short names, the own component's first: `With<Spinner>`,
    /// `Without<Enemy>`.
    pub fn filter_names(&self) -> Vec<String> {
        let own = match self.filter {
            Some(Filter::Added) => Some(format!("Added<{}>", self.own)),
            Some(Filter::With) => Some(format!("With<{}>", self.own)),
            None => None,
        };
        own.into_iter().chain(self.filters.iter().map(|f| format!("{}<{}>", f.op.rust(), f.component.name))).collect()
    }

    /// The components every entity it matches has: the ones it asks for, and the ones a `With`,
    /// `Added` or `Changed` filter is on. Bevy's `FilteredAccess` keeps the same set.
    fn requires(&self) -> BTreeSet<&str> {
        let mut out: BTreeSet<&str> =
            self.items.iter().filter(|i| i.subject != Subject::Entity).map(|i| i.name.as_str()).collect();
        if self.filter.is_some() {
            out.insert(&self.own);
        }
        out.extend(self.filters.iter().filter(|f| f.op != FilterOp::Without).map(|f| f.component.name.as_str()));
        out
    }

    /// Whether no entity can match both: one has `Without<T>` and the other requires `T`.
    pub fn disjoint(&self, other: &System) -> bool {
        let excludes = |a: &System, b: &System| {
            let required = b.requires();
            a.filters.iter().any(|f| f.op == FilterOp::Without && required.contains(f.component.name.as_str()))
        };
        excludes(self, other) || excludes(other, self)
    }

    /// Its parameters' types, in the order the generated function takes them: the resources,
    /// `Commands`, then the query.
    pub fn params(&self) -> Vec<String> {
        let mut params: Vec<String> = self.resources.iter().map(|r| format!("Res<{}>", r.rust())).collect();
        if self.commands {
            params.push("Commands".into());
        }
        params.push(self.query());
        params
    }

    /// The components it reads but doesn't write, by short name. `Added<T>` and `Changed<T>`
    /// read `T`.
    pub fn reads(&self) -> BTreeSet<&str> {
        let mut reads: BTreeSet<&str> = self
            .items
            .iter()
            .filter(|i| i.subject != Subject::Entity && !i.write)
            .map(|i| i.name.as_str())
            .collect();
        if self.filter == Some(Filter::Added) {
            reads.insert(&self.own);
        }
        reads.extend(
            self.filters
                .iter()
                .filter(|f| matches!(f.op, FilterOp::Added | FilterOp::Changed))
                .map(|f| f.component.name.as_str()),
        );
        let writes = self.writes();
        reads.retain(|r| !writes.contains(r));
        reads
    }

    /// The components it writes, by short name.
    pub fn writes(&self) -> BTreeSet<&str> {
        self.items.iter().filter(|i| i.write).map(|i| i.name.as_str()).collect()
    }
}

/// Where a system is: by script, in the order given to [`ambiguities`], then by position in
/// that script's systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct At {
    pub script: usize,
    pub system: usize,
}

/// Two systems Bevy's ambiguity detection reports: nothing orders one before the other, and one
/// writes a component the other reads or writes, so which of them runs first, and so what each
/// sees, may change from frame to frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ambiguity {
    pub a: At,
    pub b: At,
    /// The components they conflict on, by short name, sorted.
    pub components: Vec<String>,
}

/// The ambiguous pairs among several scripts' systems, each script's as [`systems`] returns them.
///
/// This is Bevy's rule (`Systems::get_conflicting_systems`) for what these systems can ask for.
/// A script's plugin chains its systems, so only systems of different scripts are unordered.
/// Two unordered systems conflict on a component one writes and the other reads or writes,
/// unless a `Without` keeps their queries from matching the same entity; `With` and `Without`
/// aren't accesses. Resources aren't compared, since every one these
/// systems take is read-only, and `Commands` only queues changes for later.
///
/// Only scripts are seen: the game's own systems in `Update` can be ambiguous with them too.
pub fn ambiguities(scripts: &[Vec<System>]) -> Vec<Ambiguity> {
    let all: Vec<(At, &System)> = scripts
        .iter()
        .enumerate()
        .flat_map(|(script, systems)| systems.iter().enumerate().map(move |(system, s)| (At { script, system }, s)))
        .collect();
    let mut out = Vec::new();
    for (i, (a, sa)) in all.iter().enumerate() {
        for (b, sb) in &all[i + 1..] {
            if a.script == b.script {
                continue;
            }
            let components = conflicts(sa, sb);
            if !components.is_empty() {
                out.push(Ambiguity { a: *a, b: *b, components });
            }
        }
    }
    out
}

/// The components two systems conflict on: one writes it, the other reads or writes it, and
/// their queries can match the same entity.
pub fn conflicts(a: &System, b: &System) -> Vec<String> {
    if a.disjoint(b) {
        return Vec::new();
    }
    let (wa, wb) = (a.writes(), b.writes());
    let (ra, rb) = (a.reads(), b.reads());
    let mut out: BTreeSet<&str> = wa.intersection(&wb).copied().collect();
    out.extend(wa.intersection(&rb));
    out.extend(wb.intersection(&ra));
    out.into_iter().map(String::from).collect()
}

/// A script's systems, in the order its plugin adds them.
pub fn systems(ir: &Module, names: &Names) -> Vec<System> {
    let transform = ir.components.iter().position(|c| c.name == "Transform");
    let ordered = ir
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.event == Event::Start)
        .chain(ir.blocks.iter().enumerate().filter(|(_, b)| b.event != Event::Start));
    let mut functions = Namer::default();
    ordered
        .map(|(index, block)| {
            let module = &names.module;
            let default = match &block.event {
                Event::Start => format!("setup_{module}"),
                Event::Update => format!("update_{module}"),
                Event::Key(key) => format!("on_{}_{module}", snake_case(key)),
            };
            let name = functions.take(block.name.as_deref().unwrap_or(&default));
            let u = usage(&block.body, transform);
            system(ir, names, name, index, block.event == Event::Start, &u, &block.filters)
        })
        .collect()
}

fn system(ir: &Module, names: &Names, name: String, block: usize, start: bool, u: &Usage, filters: &[crate::ir::Filter]) -> System {
    let item = |subject, name: &str, rust_path: &str, write| Item {
        subject,
        name: name.to_string(),
        rust_path: rust_path.to_string(),
        write,
    };
    let mut items = Vec::new();
    if u.despawn {
        items.push(item(Subject::Entity, "", "Entity", false));
    }
    if u.transform_write || u.transform_read {
        items.push(item(Subject::Transform, "Transform", "Transform", u.transform_write));
    }
    let has_own = u.own_read || u.own_write;
    if has_own {
        items.push(item(Subject::Own, &names.ty, &names.ty, u.own_write));
    }
    for (i, c) in ir.components.iter().enumerate() {
        let (read, write) = (u.fields_read.contains(&i), u.fields_write.contains(&i));
        if read || write {
            items.push(item(Subject::Component(i), &c.name, &c.rust_path, write));
        }
    }
    let filter = match (start, has_own) {
        (true, _) => Some(Filter::Added),
        (false, false) => Some(Filter::With),
        (false, true) => None,
    };
    let filters = filters.to_vec();
    System { name, block, own: names.ty.clone(), items, filter, filters, resources: u.resources.clone(), commands: u.despawn }
}

/// Whether an event's steps read or write its entity's `Transform`, so its query asks for it.
/// `transform` is the index of `Transform` in `Module::components`, if a node uses its fields.
pub fn uses_transform(steps: &[Step], transform: Option<usize>) -> bool {
    let u = usage(steps, transform);
    u.transform_read || u.transform_write
}

/// What an event's steps touch.
#[derive(Default)]
struct Usage {
    resources: BTreeSet<Resource>,
    despawn: bool,
    transform_read: bool,
    transform_write: bool,
    own_read: bool,
    own_write: bool,
    /// Other components read or written, by index into `Module::components`.
    fields_read: BTreeSet<usize>,
    fields_write: BTreeSet<usize>,
    /// The index of `Transform` in `Module::components`, whose fields count as transform use.
    transform: Option<usize>,
}

fn usage(steps: &[Step], transform: Option<usize>) -> Usage {
    fn op(o: &Operand, u: &mut Usage) {
        match o {
            Operand::Var(_) | Operand::State(_) => u.own_read = true,
            Operand::Intrinsic(Intrinsic::Delta | Intrinsic::Time) => {
                u.resources.insert(Resource::Time);
            }
            Operand::Intrinsic(Intrinsic::Button { device: Device::Key, .. }) => {
                u.resources.insert(Resource::Keys);
            }
            Operand::Intrinsic(Intrinsic::Button { device: Device::Mouse, .. }) => {
                u.resources.insert(Resource::Mouse);
            }
            Operand::Intrinsic(Intrinsic::Position(_)) => u.transform_read = true,
            Operand::Field { component, .. } if Some(*component) == u.transform => u.transform_read = true,
            Operand::Field { component, .. } => {
                u.fields_read.insert(*component);
            }
            Operand::Lit(_) | Operand::Local(_) => {}
        }
    }
    fn walk(steps: &[Step], u: &mut Usage) {
        for step in steps {
            step.lets.iter().flat_map(|l| &l.args).for_each(|a| op(a, u));
            match &step.action {
                Action::If { cond, then, otherwise } => {
                    op(cond, u);
                    walk(then, u);
                    walk(otherwise, u);
                }
                Action::Despawn => u.despawn = true,
                Action::SetVar { value, .. } | Action::SetState { value, .. } => {
                    u.own_write = true;
                    op(value, u);
                }
                Action::SetField { component, value, .. } => {
                    if Some(*component) == u.transform {
                        u.transform_write = true;
                    } else {
                        u.fields_write.insert(*component);
                    }
                    op(value, u);
                }
                Action::Effect { op: kind, args } => {
                    if *kind != EffectOp::Print {
                        u.transform_write = true;
                    }
                    args.iter().for_each(|a| op(a, u));
                }
            }
        }
    }
    let mut u = Usage { transform, ..Usage::default() };
    walk(steps, &mut u);
    u
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rust::names;

    #[test]
    fn the_starter_spins_with_time_and_its_own_speed() {
        let ir = crate::lower(&crate::starter()).module;
        let systems = systems(&ir, &names("scripts/spinner.lua"));
        assert_eq!(systems.len(), 1);
        let s = &systems[0];
        assert_eq!(s.name, "update_spinner");
        assert_eq!(s.query(), "Query<(&mut Transform, &Spinner)>");
        assert_eq!(s.resources.iter().map(|r| r.rust()).collect::<Vec<_>>(), ["Time"]);
        assert_eq!(s.writes(), BTreeSet::from(["Transform"]));
        assert_eq!(s.reads(), BTreeSet::from(["Spinner"]));
    }

    /// A spinner (writes `Transform`, reads its own component) and a door whose Start event reads
    /// `Transform`, in two scripts.
    fn two_scripts() -> Vec<Vec<System>> {
        let spinner = crate::lower(&crate::starter()).module;
        let mut door = crate::Graph::default();
        let start = door.add_node("event.start", None, 0.0, 0.0);
        let print = door.add_node("action.print", None, 200.0, 0.0);
        let position = door.add_node("transform.position", None, 0.0, 100.0);
        door.link(crate::PinRef::new(start, "then"), crate::PinRef::new(print, "in"));
        door.link(crate::PinRef::new(position, "x"), crate::PinRef::new(print, "value"));
        let door = crate::lower(&door);
        assert!(door.problems.is_empty(), "{:?}", door.problems);
        vec![systems(&spinner, &names("scripts/spinner.lua")), systems(&door.module, &names("scripts/door.lua"))]
    }

    #[test]
    fn a_write_and_a_read_in_two_scripts_are_ambiguous() {
        let scripts = two_scripts();
        assert_eq!(scripts[1][0].query(), "Query<&Transform, Added<Door>>");
        let found = ambiguities(&scripts);
        assert_eq!(
            found,
            [Ambiguity { a: At { script: 0, system: 0 }, b: At { script: 1, system: 0 }, components: vec!["Transform".into()] }]
        );
    }

    #[test]
    fn systems_of_one_script_are_chained_and_reads_never_conflict() {
        let scripts = two_scripts();
        // The same two systems in one script run in order.
        assert!(ambiguities(&[scripts.concat()]).is_empty());
        // Two readers of `Transform` don't conflict.
        assert!(ambiguities(&[scripts[1].clone(), scripts[1].clone()]).is_empty());
    }

    /// The starter's spinner with filters on its Update event.
    fn filtered(filters: &[(crate::FilterOp, &str)]) -> crate::Lowered {
        let mut graph = crate::starter();
        let update = graph.nodes.iter_mut().find(|n| n.kind == "event.update").unwrap();
        update.filters =
            filters.iter().map(|(op, c)| crate::QueryFilter { op: *op, component: c.to_string() }).collect();
        crate::lower(&graph)
    }

    #[test]
    fn event_filters_follow_the_own_components_in_the_query() {
        use crate::FilterOp::*;
        let lowered = filtered(&[(Without, "PointLight"), (Changed, "SpotLight")]);
        assert!(lowered.problems.is_empty(), "{:?}", lowered.problems);
        let s = &systems(&lowered.module, &names("scripts/spinner.lua"))[0];
        assert_eq!(s.query(), "Query<(&mut Transform, &Spinner), (Without<PointLight>, Changed<SpotLight>)>");
        // `Changed<T>` reads `T`; `Without<T>` touches nothing.
        assert_eq!(s.reads(), BTreeSet::from(["Spinner", "SpotLight"]));

        let rust = crate::rust::emit_script_traced(&crate::rust::Script { path: "scripts/spinner.lua".into(), ir: &lowered.module }).code;
        let flat: String = rust.split_whitespace().collect::<Vec<_>>().join(" ").replace("< ", "<").replace(", >", ">");
        assert!(flat.contains("Query<(&mut Transform, &Spinner), (Without<PointLight>, Changed<SpotLight>)>"), "{rust}");
        let lua = crate::lua::emit(&lowered.module);
        assert!(lua.contains(r#"if vs.filter(entity, "without", "PointLight") and vs.filter(entity, "changed", "SpotLight") then"#), "{lua}");
    }

    #[test]
    fn a_filter_that_never_matches_or_names_no_component_is_an_error() {
        use crate::FilterOp::*;
        let lowered = filtered(&[(Without, "Transform"), (With, "Nope")]);
        let messages: Vec<&str> = lowered.problems.iter().map(|p| p.message.as_str()).collect();
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("Unknown component `Nope`")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("`Without<Transform>` never matches")), "{messages:?}");
    }

    #[test]
    fn a_without_keeps_two_systems_from_conflicting() {
        use crate::FilterOp::*;
        let mut scripts = two_scripts();
        assert_eq!(ambiguities(&scripts).len(), 1);
        // The door's Start reads `Transform` of entities with `Door`; the spinner skips those.
        let spinner = filtered(&[(Without, "SpotLight")]).module;
        let mut s = systems(&spinner, &names("scripts/spinner.lua"));
        s[0].filters[0].component.name = "Door".into();
        scripts[0] = s;
        assert!(ambiguities(&scripts).is_empty());
    }

    #[test]
    fn a_start_event_reads_its_component_through_added() {
        let mut graph = crate::Graph::default();
        graph.add_node("event.start", None, 0.0, 0.0);
        let ir = crate::lower(&graph).module;
        let s = &systems(&ir, &names("scripts/door.lua"))[0];
        assert_eq!(s.query(), "Query<(), Added<Door>>");
        assert_eq!(s.reads(), BTreeSet::from(["Door"]));
        assert!(s.writes().is_empty());
    }
}
