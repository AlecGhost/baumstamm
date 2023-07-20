<script lang="ts">
	import type { Connections } from '../bindings';

	export let connections: Connections;

	function y(connection: number): number {
		return ((connection + 1) / (connections.total + 1)) * 100;
	}

	function xEnding(index: number): number {
		return ((index + 1) / (connections.ending.length + 1)) * 100;
	}

	function xCrossing(index: number): number {
		return ((index + 1) / (connections.crossing.length + 1)) * 100;
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
			style="stroke:rgb(0, 0, 0);stroke-width: 5;"
		/>
	{/each}
	{#each connections.ending as ending, i}
		<!-- horizontal line -->
		{#if ending.origin == 'Left'}
			<line
				x1="0%"
				y1="{y(ending.connection)}%"
				x2="{xEnding(i)}%"
				y2="{y(ending.connection)}%"
				style="stroke:rgb(0, 0, 0);stroke-width: 5;"
			/>
		{:else if ending.origin == 'Right'}
			<line
				x1="100%"
				y1="{y(ending.connection)}%"
				x2="{xEnding(i)}%"
				y2="{y(ending.connection)}%"
				style="stroke:rgb(0, 0, 0);stroke-width: 5;"
			/>
		{/if}
		<!-- vertical line -->
		<line
			x1="{xEnding(i)}%"
			y1="{connections.orientation == 'Up' ? 0 : 100}%"
			x2="{xEnding(i)}%"
			y2="{y(ending.connection)}%"
			style="stroke:rgb(0, 0, 0);stroke-width: 5;"
		/>
	{/each}
	{#each connections.crossing as crossing, i}
		<!-- horizontal line -->
		{#if crossing.origin == 'Left'}
			<line
				x1="0%"
				y1="{y(crossing.connection)}%"
				x2="{xCrossing(i)}%"
				y2="{y(crossing.connection)}%"
				style="stroke:rgb(0, 0, 0);stroke-width: 5;"
			/>
		{:else if crossing.origin == 'Right'}
			<line
				x1="100%"
				y1="{y(crossing.connection)}%"
				x2="{xCrossing(i)}%"
				y2="{y(crossing.connection)}%"
				style="stroke:rgb(0, 0, 0);stroke-width: 5;"
			/>
		{/if}
		<!-- vertical line -->
		<line
			x1="{xCrossing(i)}%"
			y1="{connections.orientation == 'Up' ? 100 : 0}%"
			x2="{xCrossing(i)}%"
			y2="{y(crossing.connection)}%"
			style="stroke:rgb(0, 0, 0);stroke-width: 5;"
		/>
	{/each}
</svg>
