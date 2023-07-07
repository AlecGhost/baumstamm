<script lang="ts">
	import panzoom from 'panzoom';

	function initPanzoom(node: HTMLElement) {
		panzoom(node);
	}
</script>

<div class="h-full w-full tree-view" use:initPanzoom>
	<!-- TODO: populate with nodes -->
</div>

<style>
	.tree-view {
		user-select: none;
		position: absolute;
		cursor: move;
	}
</style>
