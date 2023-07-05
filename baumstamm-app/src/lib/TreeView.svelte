<script lang="ts">
	// movement
	let x_pos = 0;
	let y_pos = 0;
	let moving = false;

	function move(e: MouseEvent) {
		if (moving) {
			x_pos -= e.movementX;
			y_pos -= e.movementY;
		}
	}

	function setMoving() {
		moving = true;
	}

	function unsetMoving() {
		moving = false;
	}

	// zoom
	let zoom = 1000;
	let mouseover = false;

	function wheel(e: WheelEvent) {
		if (mouseover) {
			let rel_x = x_pos / zoom;
			let rel_y = y_pos / zoom;
			zoom += e.deltaY;
			x_pos = zoom * rel_x;
			y_pos = zoom * rel_y;
		}
	}

	function setMouseover() {
		mouseover = true;
	}

	function unsetMouseover() {
		mouseover = false;
	}
</script>

<svelte:window on:mouseup={unsetMoving} on:mousemove={move} on:wheel={wheel} />

<div
	class="h-full w-full tree-view"
	on:mousedown={setMoving}
	on:mouseover={setMouseover}
	on:mouseout={unsetMouseover}
>
	<svg viewBox="{x_pos} {y_pos} {zoom} {zoom}">
        <!-- TODO: populate with nodes -->
	</svg>
</div>

<style>
	.tree-view {
		user-select: none;
		position: absolute;
		cursor: move;
	}
</style>
