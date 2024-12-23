addEventListener("TrunkApplicationStarted", (event) => {
	console.log("application started - bindings:", window.wasmBindings, "WASM:", event.detail.wasm);
	window.wasmBindings.greet('World');
});
