<script>
	import App from '$lib/App.svelte';
	import { init_state } from '$lib/baumstamm-wasm/baumstamm_wasm';

	if (!('__TAURI__' in window)) {
		window.state = init_state();
	}
</script>

<App />
