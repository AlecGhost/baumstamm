<script lang="ts">
	import type { Connections } from '$lib/api';

	export let connections: Connections;

	function y(connection: number): number {
		return ((connection + 1) / (connections.total_y + 1)) * 100;
	}

	function x(index: number): number {
		return ((index + 1) / (connections.total_x + 1)) * 100;
	}
</script>

<svg width="100%" height="100%">
	{#each connections.passing as passing}
		<!-- horizontal line -->
		<line
			x1="0%"
			y1="{y(passing.connection)}%"
			x2="100%"
			y2="{y(passing.connection)}%"
			class={passing.connection.toString()}
			style="--color: {passing.color[0]}, {passing.color[1]}%, {passing.color[2]}%"
		/>
	{/each}
	{#each connections.ending as ending}
		<!-- horizontal line -->
		{#if ending.origin == 'Left'}
			<line
				x1="0%"
				y1="{y(ending.connection)}%"
				x2="{x(ending.x_index)}%"
				y2="{y(ending.connection)}%"
				class={ending.connection.toString()}
				style="--color: {ending.color[0]}, {ending.color[1]}%, {ending.color[2]}%"
			/>
		{:else if ending.origin == 'Right'}
			<line
				x1="100%"
				y1="{y(ending.connection)}%"
				x2="{x(ending.x_index)}%"
				y2="{y(ending.connection)}%"
				class={ending.connection.toString()}
				style="--color: {ending.color[0]}, {ending.color[1]}%, {ending.color[2]}%"
			/>
		{/if}
		<!-- vertical line -->
		<line
			x1="{x(ending.x_index)}%"
			y1="{connections.orientation == 'Up' ? 0 : 100}%"
			x2="{x(ending.x_index)}%"
			y2="{y(ending.connection)}%"
			class={ending.connection.toString()}
			style="--color: {ending.color[0]}, {ending.color[1]}%, {ending.color[2]}%"
		/>
	{/each}
	{#each connections.crossing as crossing}
		<!-- horizontal line -->
		{#if crossing.origin == 'Left'}
			<line
				x1="0%"
				y1="{y(crossing.connection)}%"
				x2="{x(crossing.x_index)}%"
				y2="{y(crossing.connection)}%"
				class={crossing.connection.toString()}
				style="--color: {crossing.color[0]}, {crossing.color[1]}%, {crossing.color[2]}%"
			/>
		{:else if crossing.origin == 'Right'}
			<line
				x1="100%"
				y1="{y(crossing.connection)}%"
				x2="{x(crossing.x_index)}%"
				y2="{y(crossing.connection)}%"
				class={crossing.connection.toString()}
				style="--color: {crossing.color[0]}, {crossing.color[1]}%, {crossing.color[2]}%"
			/>
		{/if}
		<!-- vertical line -->
		<line
			x1="{x(crossing.x_index)}%"
			y1="{connections.orientation == 'Up' ? 100 : 0}%"
			x2="{x(crossing.x_index)}%"
			y2="{y(crossing.connection)}%"
			class={crossing.connection.toString()}
			style="--color: {crossing.color[0]}, {crossing.color[1]}%, {crossing.color[2]}%"
		/>
	{/each}
</svg>

<style>
	line {
		stroke-width: 5;
		stroke: hsl(var(--color));
	}
</style>
