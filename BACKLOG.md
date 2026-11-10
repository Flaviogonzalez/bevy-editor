# Backlog

Cosas para iterar en el editor. Cada entrada dice qué pasa y, cuando se sabe, por qué.

## Plan principal

**Qué es bevy-editor.** Escribir un juego Bevy completo de forma visual, con la estructura de Bevy
a la vista (plugins, systems, queries, componentes, recursos), cubriendo todo Bevy, y con un
resultado que sea el mismo Rust nativo que escribiría alguien que sabe Bevy. Lo que lo distingue:

- El grafo compila a Rust nativo que parece escrito a mano (ya funciona; lo verifica el test
  diferencial). Blueprints corre en una máquina virtual.
- La estructura de ECS se ve y se crea, en vez de deducirse y esconderse.
- Como el resultado es nativo, las herramientas profesionales (debugger, profiler, ensamblador)
  se aplican, con trazabilidad hasta el nodo.

**Orden.** El riesgo es de alcance, no técnico, así que importa más el orden que las ideas:

1. **Estructura.** Grupos plegables que son systems, queries explícitas y editables, plugins.
   Casi todo lo demás se apoya acá.
2. **Un juego chico de verdad como guía.** Agregar lo que ese juego pida, en vez de cubrir Bevy
   en abstracto. Candidato: un juego de la comunidad con licencia MIT/Apache y de la misma versión
   de Bevy, por ejemplo los de `examples/games` del repo de Bevy (breakout, alien_cake_addict).
   La versión original escrita a mano sirve de referencia para el test diferencial y para el
   benchmark en fps.
3. **Funciones**, con acceso completo a componentes y recursos.
4. **Trazabilidad barata.** Guardar qué líneas genera cada nodo; habilita después breakpoints
   por nodo (Debug Adapter Protocol), ensamblador por nodo y costo por system (tracing de Bevy,
   Tracy).
5. **Lo grande, después:** importar la estructura de proyectos existentes (con `syn`), el
   inspector en vivo (Bevy Remote Protocol) y la escena (en el formato que adopte Bevy, sin
   inventar uno). El grafo de todas las primitivas de Rust con sincronización en las dos
   direcciones va al final: base al nivel del lenguaje tal como se escribe (no MIR), abstracciones
   encima, llamadas como nodos (no como cuerpos expandidos), lifetimes deducidos y mostrados solo
   como errores.

**Vista previa.** Se queda en Lua. `experiments/hotpatch_probe` mostró que el hot patching de Rust
solo aplica cambios dentro del cuerpo de un system. Agregar un system o cambiar un componente pide
recompilar y reiniciar (y un cambio de componente sin reinicio corrompe memoria sin aviso), y no
hay trazabilidad de qué cambió. Como cada nodo se escribe una sola vez en Rust y Lua solo lo
llama, el costo doble está en las partes estructurales de los dos backends (queries, componentes,
recursos), no en cada nodo.

## Pedidos

- **Rendimiento con muchos nodos.** Con muchos nodos en el lienzo la ventana se vuelve lenta.
  Medido con 400 nodos: en release cada cuadro tardaba ~50 ms y en debug ~300 ms, y casi todo era
  GPUI haciendo layout y pintando nodos fuera de pantalla. Ahora solo los nodos visibles son
  elementos (~25 ms en release). Queda:
  - Cada cuadro (cada movimiento del mouse) vuelve a compilar el grafo, buscar ambigüedades y
    rehacer el layout de todos los nodos: ~4 ms en release, ~15 ms en debug. Se puede guardar el
    resultado hasta que el grafo cambie.
  - En debug las dependencias (GPUI) se compilan sin optimizar; `opt-level` en
    `[profile.dev.package."*"]` lo arreglaría a costa de una recompilación.
  - Con zoom muy alejado todos los nodos vuelven a estar en vista; ahí convendría dibujarlos
    simplificados.
  - Sigue andando mal con el script `coverage` en el proyecto compcheck, aun con el recorte.
  - Medirlo en **fps**, usando el benchmark del usuario como referencia (no en ms por cuadro con
    un script sintético).
- **Nada de Lua fuera de debug.** Lua solo importa para depurar: el usuario no participa del build
  en Lua, entonces no tiene sentido que lo vea. En release no debería aparecer nada de Lua: ni la
  vista "Lua (preview)", ni la extensión `.lua` en títulos, pestañas, mensajes o diálogos.
  **Decidido:** solo se esconde en la interfaz; el archivo sigue siendo `.lua` por dentro, porque
  al usuario le importa que funcione, no qué pasa de fondo. Lugares a tocar:
  - `panels/bottom.rs`: el selector "Lua (preview)" / "Rust (build)" (fuera de debug, solo Rust).
  - `Document::file_name()`: título de la ventana, nombre en la barra, diálogo de cambios sin
    guardar → el nombre sin extensión.
  - Mensajes de estado con rutas completas (`Opened …\demo.lua`, `Saved …`, `Couldn't …`) en
    `editor.rs` y `workspace/project.rs` → el nombre del script.
  - Las ambigüedades citan `(watch/watcher.lua)` → `watch/watcher`.
  - Diálogos de abrir y guardar: filtro "Visual script" y nombre sugerido `script.lua`; el
    diálogo del sistema va a seguir mostrando `.lua`.
  - Revisar si la salida de la vista previa (logs de Bevy) menciona Lua.

- **Reestructurar los demás crates como se hizo con vs_editor.** En el editor pasaba que
  `render.rs` había juntado 1589 líneas con responsabilidades distintas (primitivas de UI, layout,
  lienzo, paneles), y `Editor` tenía unos veinte campos sueltos. Se arregló en `f62814e` (separar
  `render.rs` en `ui`, `canvas` y `panels`, como movida pura sin cambiar comportamiento) y en
  `a5a6517` (agrupar el estado en `Document`, `CanvasState`, `Chrome` y `Workspace` según a quién
  pertenece y por qué cambia). Los otros crates probablemente tienen el mismo problema. Candidatos por
  tamaño, a revisar uno por uno antes de mover nada:
  - `vs_graph/src/rust.rs`: 1239 líneas y ~50 funciones; mezcla nombres, árbol de carpetas y
    plugins, imports, queries y las dos salidas (dev y export).
  - `vs_graph/src/lower.rs` (545) y `graph.rs` (427).
  - `vs_project/src/lib.rs` (498): configuración, creación de proyectos, scripts, build y run en
    un solo archivo.
  - `vs_bevy/src/lib.rs` (278) y `vs_nodes/src/lib.rs` (381), más chicos pero a mirar.
  La forma de trabajo que funcionó: primero una revisión que proponga los módulos (responsabilidad,
  entradas, salidas), después una movida pura por commit, y recién ahí cambios de comportamiento.

- **Que la UI muestre cómo funciona de verdad un proyecto Bevy.** Hoy el editor sigue el modelo
  de Blueprints/Unity: un script es un archivo con eventos, cada evento se vuelve un system sin que
  el usuario lo vea, la query se deduce de lo que usa el grafo y la carpeta se vuelve plugin. Eso
  esconde el ECS. La idea es una estructura `[plugin]/[system]/components`, donde el system es
  algo que el usuario crea, y cada system tiene sus queries a la vista. Las queries también se
  tienen que poder crear, así que hay que validar a fondo qué parte de las queries de Bevy se
  soporta: `&T`, `&mut T`, `Option<&T>`, `Entity`; los filtros `With`, `Without`, `Added`,
  `Changed` y `Or`; varias queries por system (ahí aparece B0001); los recursos y `Commands`.
  Preguntas abiertas:
  - ¿Un grafo por system (como una función de Bevy) o un grafo por plugin con varios systems?
  - ¿La query se escribe explícita, se deduce y se puede ajustar, o las dos cosas?
  - ¿Dónde se declaran los componentes: en el plugin, globales o en ambos?
  - El orden entre systems (`before`/`after`, sets) y el schedule (`Startup`, `Update`), ¿se
    muestran y se editan?
  - ¿Qué pasa con los scripts que ya existen?

  **Idea del usuario: grupos como los comment boxes de Unreal.** No un grafo por system, sino una
  "tapa transparente" que cubre un conjunto de nodos y los agrupa: el grupo es el system. Se puede
  expandir o plegar, y un grupo puede cubrir otros grupos (un plugin que cubre sus systems y sus
  componentes). Cómo encaja:
  - Hoy el archivo ya contiene varios systems (uno por evento), y el análisis sabe qué nodos
    pertenecen a cada uno (`Block.node`, `access::systems`). Entonces el primer paso puede ser
    dibujar esa tapa alrededor de cada evento y sus nodos, con nombre, query y schedule en la
    cabecera, sin cambiar el formato.
  - Plegado, el system se ve como una tarjeta (nombre, query, parámetros); esto además ayuda al
    rendimiento y al zoom alejado.
  - Pertenencia explícita (el nodo es del grupo y la tapa se ajusta sola) en vez de geométrica como
    en Unreal, donde arrastrar un nodo por encima lo mete sin querer.
  - Un system no comparte variables locales con otro: un cable que cruza de un grupo a otro es un
    error, y los datos entre systems pasan por componentes o recursos.

  **Primer paso, implementado y sin revisar en la app: la tapa de cada evento, sin cambiar el
  formato.** El `Lowerer` anota los nodos de cada evento en `Block.nodes`, y `canvas/groups.rs`
  arma con eso un marco por system (cabecera con orden, nombre, schedule y parámetros) y la
  franja de cada nodo. El pie del evento ya no existe.
  - **Decidido:** un nodo de datos que alimenta a dos systems queda fuera de toda tapa, con
    aspecto neutro y sin advertencia (hoy es legal: el código generado ya lo evalúa en cada
    system). El error llega cuando la pertenencia sea explícita, junto con una acción de
    "duplicar en cada system".
  - **Decidido:** cada nodo lleva una franja con el color de su system, para que la pertenencia
    no dependa de la geometría (el rectángulo de una tapa puede cubrir nodos ajenos o solaparse
    con otra). El nodo compartido no lleva franja.
  - Abierto: qué schedule mostrar para un evento Start, que en realidad corre en `Update` con
    `Added<T>`. Por ahora la cabecera dice `Update`, porque es lo que hace el código; falta
    que el usuario lo vea y decida.
  - Fuera de este paso: arrastrar la cabecera para mover el grupo, plegar, cambios al formato.

  **Las otras dos piezas de cada script, que el lienzo no muestra.** Cada script genera tres
  cosas: sus systems (ya son marcos), su componente propio (`LightBehavior`) y su plugin
  (`LightBehaviorPlugin`, que hace `add_systems`). **Acordado:** el plugin sería el marco de
  afuera, con sus systems adentro, y el componente propio una tarjeta como las de Define
  Component, con las variables como campos y los `#[require]` a la vista. Un componente sin
  campos es válido y útil: es un marcador, como los que Bevy usa para filtrar una query con
  `With<T>`, así que la tarjeta no puede dar por hecho que tiene campos. Va antes que las queries
  editables, porque toda query filtra por ese componente y editarla sin verlo esconde el ECS.

  **Idea del usuario: el plugin reemplaza al script como unidad.** El usuario no quiere ver ni
  editar scripts (ni Lua) conceptualmente, sino su plugin y lo que contiene. En el Explorer,
  cada plugin se abre como un schema en un cliente SQL: carpetas por tipo de pieza
  (Components, Systems, Resources, y lo que Bevy tenga), con X de cada una. Consecuencias a
  iterar:
  - Por dentro sigue habiendo un archivo por plugin (ya decidido: Lua solo se esconde en la
    interfaz). El primer paso puede ser solo de presentación: el árbol se deduce del grafo
    (Systems = eventos, Components = el propio y los Define Component) y un clic lleva al
    marco o a la tarjeta en el lienzo.
  - Hoy hay dos tipos de plugin: el de una carpeta (solo agrupa sub-plugins) y el de un archivo
    (tiene systems y componentes). ¿Se unifican en "todo plugin es una carpeta", con su grafo y
    sus sub-plugins adentro?
  - **Decidido:** el componente propio (`LightBehavior`) deja de ser especial y pasa a ser un
    componente más del plugin. Venía del modelo "script pegado a una entidad", y en un plugin
    de Bevy ningún componente es privilegiado. Con eso se cae la tarjeta fija propuesta antes.
    Consecuencias pendientes:
    - El `With<T>` de cada query pasa a ser una elección explícita, así que esto se implementa
      junto con las queries editables, no antes: sin elegir el filtro, un system no tendría
      forma de decir sobre qué entidades actúa.
    - Las variables dejan de ser "del script" y pasan a ser campos de un componente elegido.
    - El estado de Flip Flop y Do Once tiene que elegir entre un campo de componente (uno por
      entidad) o `Local<T>` (uno por system). Bevy tiene los dos y significan distinto.
    - Scripts existentes: su componente propio se convierte en un componente declarado común,
      con los mismos campos, y sus queries quedan filtrando por él, así no cambia lo que hacen.
  - Resources no existe todavía: la carpeta marca lo que falta para cubrir Bevy.
  Reemplaza a la propuesta de una barra de plugins arriba del lienzo.

- **Funciones.** Poder armar una función con nodos y usarla como un nodo más. Dos formas, que
  Unreal tiene por separado:
  - *Nodo plegable* (como los collapsed graphs): organiza un pedazo del grafo dentro del mismo
    script. Es lo mismo que plegar un grupo, así que sale junto con los grupos.
  - *Función reutilizable que se abre en otra pestaña*, con entradas y salidas declaradas. Se
    usa desde varios scripts y se compila a una `fn` de Rust (y de Lua en la vista previa). Las
    pestañas ya existen.
  Las funciones leen y escriben componentes desde el principio: la meta es cubrir todo Bevy, y eso
  es parte de la estructura principal. Lo que usa una función se suma a la query (y a los
  recursos) del system que la llama, así que `access` tiene que componer los accesos de las
  funciones llamadas, y las ambigüedades y conflictos tienen que verlos.
- **Helpers de diseño: grupos de nodos prefabricados.** Plantillas que insertan un grupo armado
  para un patrón común. No hace falta inventar abstracciones nuevas: cada helper crea piezas de
  Bevy (componentes, systems, recursos, plantillas de spawn) ya armadas para ese patrón:
  - Singleton → un `Resource`, o una query `Single` si es una entidad única.
  - Factory/builder → una plantilla de spawn: una entidad con sus componentes y los `#[require]`
    que ya generamos.
  - Concurrencia → Bevy ya corre los systems en paralelo cuando no chocan (lo que muestran las
    ambigüedades). Lo que se puede ofrecer es `par_iter` dentro de un system y tareas async.
- **Benchmark de rendimiento contra un juego escrito a mano** (en fps, con muchas entidades), para
  poder afirmar que el export rinde igual. Requiere un juego de verdad, así que va después de
  poder armar una demo coherente.

- **Remarcar en el código Rust lo que genera el nodo seleccionado.** Implementado: cada `Step`
  del IR lista sus nodos, el emisor intercala marcadores (`__vs_node!(4);`), formatea, los
  quita y devuelve un mapa nodo → rangos de líneas (`rust::emit_script_traced`). Es la base de
  la "trazabilidad barata" del plan (punto 4). Falta: la línea de `add_systems` no se marca
  (es un elemento de una tupla, donde no entra un marcador), y la vista Lua no resalta nada.

- **"On Update" no dice que es el comienzo de un system.** Al usuario le costó entender que el
  nodo de evento es el inicio de la función. En Bevy no hay eventos de ese tipo: hay una
  función registrada en un schedule (y On Key ya compila a `.run_if(input_just_pressed(…))`).
  Propuesta: el nodo se ve como la entrada de la función, como el nodo de entrada de una
  función en Unreal: el título es el nombre del system (`fn update_light`, el mismo que la
  cabecera del marco), y debajo lo que hace que corra (`every frame in Update`,
  `when LightBehavior is added`, `run_if: Space just pressed`). El pin `delta` dice de dónde
  sale (`Res<Time>`). Alternativa más grande, para después: sacar el nodo y que la cabecera
  del marco sea la entrada, con el pin de ejecución saliendo de ella.

## Limitaciones conocidas

- **Ambigüedades con pestañas sin guardar.** El análisis de ambigüedades lee los otros scripts del
  disco, entonces no ve los cambios sin guardar de las otras pestañas.
- **Cerrar la ventana no pregunta.** Si hay pestañas con cambios, se pierden sin aviso.
- **No se pueden mover ni renombrar scripts** entre carpetas (es decir, entre plugins).
- **Proyectos viejos no recorren subcarpetas.** El `build.rs` de los proyectos creados antes de
  que las carpetas fueran plugins solo mira la carpeta de arriba.
- **El análisis no ve los sistemas del juego.** Solo conoce los scripts, así que una ambigüedad con
  un sistema escrito a mano no aparece.
- **B0001 no se emite.** Hoy cada sistema tiene una sola query, por eso el conflicto de dos queries
  del mismo sistema no puede ocurrir; cuando un sistema tenga varias, hay que reportarlo.
- **La cabecera de un grupo puede tapar nodos ajenos.** El marco de cada system se calcula
  alrededor de sus nodos y la cabecera va arriba, así que un nodo de otro system (o uno
  compartido) que esté justo encima queda debajo de ella.
