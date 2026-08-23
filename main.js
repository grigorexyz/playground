import init from "./pkg/wasm_app.js";

init().catch((error) => {
  console.error("Failed to initialize wasm module:", error);
});
