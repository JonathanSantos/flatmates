// Página do Flatmates: todas as ilustrações "ao vivo" saem do mesmo motor
// do binário, compilado para WebAssembly.
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];
const enc = new TextEncoder();
const dec = new TextDecoder();
const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
let wasm;

/** Renderiza uma cena. `flags`: 1 desenha o esqueleto, 2 mantém o enquadramento entre quadros. */
function render(spec, time = 0, flags = 0) {
  const bytes = enc.encode(typeof spec === "string" ? spec : JSON.stringify(spec));
  const input = wasm.flatmates_alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, input, bytes.length).set(bytes);
  const out = wasm.flatmates_frame(input, bytes.length, time, flags);
  wasm.flatmates_free(input, bytes.length);
  const len = new DataView(wasm.memory.buffer).getUint32(out, true);
  const text = dec.decode(new Uint8Array(wasm.memory.buffer, out + 4, len));
  wasm.flatmates_free(out, len + 4);
  return JSON.parse(text);
}

/** Uma ilustração viva: redesenha quando o estado muda e, se tiver movimento, a cada quadro enquanto está na tela. */
class Live {
  constructor(root, build, { period = 1600 } = {}) {
    this.root = root;
    this.stage = $(".stage", root);
    this.build = build;
    this.period = period;
    this.rig = false;
    this.moving = false;
    this.visible = false;
    this.last = 0;
    new IntersectionObserver(([entry]) => {
      this.visible = entry.isIntersecting;
      if (this.visible) this.tick(performance.now());
    }, { rootMargin: "120px" }).observe(root);
  }
  draw(time = 0) {
    const started = performance.now();
    const reply = render(this.build(), time, (this.rig ? 1 : 0) | (this.moving ? 2 : 0));
    if (reply.svg) this.stage.innerHTML = reply.svg;
    const meter = $(".ms", this.root);
    if (meter) meter.textContent = `${(performance.now() - started).toFixed(1)} ms no seu navegador`;
    return reply;
  }
  tick(now) {
    if (!this.visible || !this.moving || still) return;
    if (now - this.last > 40) {
      this.last = now;
      this.draw((now % this.period) / this.period);
    }
    requestAnimationFrame((t) => this.tick(t));
  }
  refresh() {
    this.draw(0);
    if (this.moving) this.tick(performance.now());
  }
}

/** Grupo de botões de escolha única. */
function chips(root, name, onPick) {
  const group = $(`[data-chips="${name}"]`, root);
  const buttons = $$("button", group);
  buttons.forEach((b) => {
    b.addEventListener("click", () => {
      buttons.forEach((o) => o.setAttribute("aria-pressed", String(o === b)));
      onPick(b.dataset.value);
    });
  });
}

const quiet = { shapes: false, confetti: false };
const bare = { blob: false, shapes: false, confetti: false };

function hero() {
  const root = $("#hero");
  const state = { yaw: 24, pitch: 16, theme: "day" };
  const bands = { day: "#dce9ff", pastel: "#f3eeff", sunset: "#ffecdc", mint: "#e0f6ee", night: "#171b3d" };
  const live = new Live(root, () => ({
    theme: state.theme,
    camera: { yaw: state.yaw, pitch: state.pitch },
    size: [1400, 640],
    seed: 5,
    backdrop: quiet,
    items: [
      { id: "desk", kind: "desk", at: [0, 0] },
      { id: "laptop", kind: "laptop", on: "desk", at: [-0.25, -0.16], facing: 180, content: "code" },
      { kind: "mug", on: "desk", at: [0.3, 0.12], color: "coral" },
      { kind: "books", on: "desk", at: [0.52, -0.16] },
      { id: "chair", kind: "chair", at: [-0.25, -0.6], color: "violet" },
      { kind: "person", sits_on: "chair", uses: "laptop", hair: "bun", outfit: "sweater", skin: "teal", top_color: "yellow", hair_color: "brown" },
      { kind: "person", at: [1.5, 0.4], motion: "wave", facing: -40, hair: "short", outfit: "blazer", skin: "lilac", top_color: "pink", face: "smile" },
      { kind: "person", at: [-1.9, 0.9], motion: "walk", facing: 75, hair: "ponytail", outfit: "tshirt", skin: "peach", top_color: "blue", holds: "mug" },
      { kind: "plant", at: [-1.45, -0.45], scale: 1.1 },
      { kind: "cat", at: [0.9, 1.1], facing: -100 },
      { kind: "sofa", at: [2.4, -1.0], facing: -30, color: "coral" },
    ],
  }));
  live.moving = true;
  for (const key of ["yaw", "pitch"]) {
    const input = $(`[name="${key}"]`, root);
    input.addEventListener("input", () => {
      state[key] = Number(input.value);
      input.nextElementSibling.textContent = `${input.value}°`;
      if (still) live.draw(0);
    });
  }
  chips(root, "theme", (value) => {
    state.theme = value;
    const style = document.documentElement.style;
    style.setProperty("--band", bands[value]);
    style.setProperty("--on-band", value === "night" ? "#f2f4ff" : "#1e2444");
    if (still) live.draw(0);
  });
  $('[name="rig"]', root).addEventListener("change", (e) => {
    live.rig = e.target.checked;
    if (still) live.draw(0);
  });
  live.refresh();
}

function rigLab() {
  const root = $("#lab-rig");
  const state = { pose: "walk", facing: 70 };
  const motions = ["walk", "run", "wave", "jump"];
  const live = new Live(root, () => {
    const person = { kind: "person", facing: state.facing, hair: "bob", outfit: "tshirt", bottom: "shorts", skin: "sky", top_color: "coral", height: 1.72 };
    if (motions.includes(state.pose)) person.motion = state.pose;
    else person.pose = state.pose;
    return { theme: "day", size: [900, 560], seed: 3, backdrop: bare, items: [person] };
  }, { period: 1300 });
  live.rig = true;
  const sync = () => {
    live.moving = motions.includes(state.pose);
    live.refresh();
  };
  chips(root, "pose", (value) => { state.pose = value; sync(); });
  const facing = $('[name="facing"]', root);
  facing.addEventListener("input", () => { state.facing = Number(facing.value); if (!live.moving || still) live.draw(0); });
  $('[name="rig"]', root).addEventListener("change", (e) => { live.rig = e.target.checked; if (!live.moving || still) live.draw(0); });
  sync();
}

function headLab() {
  const root = $("#lab-head");
  const state = { facing: 35, hair: "bob" };
  const live = new Live(root, () => ({
    theme: "pastel", size: [900, 520], seed: 3, backdrop: bare,
    items: [-1, 0, 1].map((k) => ({
      kind: "person", at: [k * 0.95, 0], facing: state.facing + k * 60, hair: state.hair, face: "smile",
      hair_color: ["auburn", "ink", "blonde"][k + 1], skin: ["peach", "sky", "amber"][k + 1],
      outfit: "tshirt", top_color: ["violet", "coral", "green"][k + 1], height: 1.72,
    })),
  }));
  chips(root, "hair", (value) => { state.hair = value; live.draw(); });
  const facing = $('[name="facing"]', root);
  facing.addEventListener("input", () => { state.facing = Number(facing.value); live.draw(); });
  live.draw();
}

function clothLab() {
  const root = $("#lab-cloth");
  const state = { build: "curvy", outfit: "tank" };
  const live = new Live(root, () => ({
    theme: "sunset", size: [900, 520], seed: 4, backdrop: bare,
    items: [20, 90, 180].map((facing, k) => ({
      kind: "person", at: [(k - 1) * 0.95, 0], facing, build: state.build, outfit: state.outfit,
      bottom: state.outfit === "tank" ? "shorts" : "pants", hair: "short", skin: "lilac", top_color: "coral", height: 1.72, pose: "standing",
    })),
  }));
  chips(root, "build", (value) => { state.build = value; live.draw(); });
  chips(root, "outfit", (value) => { state.outfit = value; live.draw(); });
  live.draw();
}

function finishLab() {
  const root = $("#lab-finish");
  const state = { shading: true, haze: true, grain: false };
  const live = new Live(root, () => ({
    theme: "mint", camera: "three-quarter", size: [900, 520], seed: 6, shading: state.shading, haze: state.haze,
    backdrop: { ...quiet, grain: state.grain },
    items: [
      { id: "sofa", kind: "sofa", at: [0, -1.6], color: "violet" },
      { kind: "person", sits_on: "sofa", seat: 0, hair: "bob", outfit: "sweater", skin: "peach", top_color: "yellow" },
      { kind: "lamp", at: [1.5, -1.9] },
      { kind: "person", at: [-0.9, 0.9], pose: "pointing", facing: 30, hair: "afro", outfit: "hoodie", skin: "sky", top_color: "coral" },
      { kind: "person", at: [0.9, 1.2], pose: "thinking", facing: -40, hair: "short", outfit: "blazer", skin: "lilac", top_color: "green" },
      { kind: "plant", at: [-2.0, -1.4], scale: 1.2 },
    ],
  }));
  for (const key of Object.keys(state)) {
    $(`[name="${key}"]`, root).addEventListener("change", (e) => { state[key] = e.target.checked; live.draw(); });
  }
  live.draw();
}

function editor() {
  const root = $("#editor");
  const area = $("textarea", root);
  const notes = $(".notes", root);
  const stage = $(".stage", root);
  const update = () => {
    const reply = render(area.value);
    notes.replaceChildren();
    const note = (kind, text) => {
      const li = document.createElement("li");
      li.className = kind;
      li.textContent = text;
      notes.append(li);
    };
    if (reply.error) return note("err", reply.error);
    stage.innerHTML = reply.svg;
    reply.warnings.forEach((w) => note("warn", w));
  };
  let timer;
  area.addEventListener("input", () => { clearTimeout(timer); timer = setTimeout(update, 120); });
  const load = async (name) => {
    area.value = await (await fetch(`examples/${name}.json`)).text();
    update();
  };
  $$("[data-example]").forEach((b) => b.addEventListener("click", async () => {
    await load(b.dataset.example);
    root.scrollIntoView({ behavior: still ? "auto" : "smooth", block: "start" });
  }));
  load("custom");
}

try {
  ({ instance: { exports: wasm } } = await WebAssembly.instantiateStreaming(fetch("flatmates.wasm"), {}));
  [hero, rigLab, headLab, clothLab, finishLab, editor].forEach((start) => start());
} catch (error) {
  // Sem WebAssembly a página continua legível: as figuras vivas caem para imagens prontas.
  document.documentElement.classList.add("no-wasm");
  console.error(error);
}
