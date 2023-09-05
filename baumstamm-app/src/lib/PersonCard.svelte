<script lang="ts">
	import { Avatar } from '@skeletonlabs/skeleton';
	import type { Person } from '$lib/Person';
	import { selected, target, settings } from '$lib/store';

	export let person: Person;

	function select() {
		selected.update(() => person);
	}
</script>

<div class="person-card m-0">
	<div
		class="card p-4 mx-4 h-full"
		class:variant-filled-primary={$selected == person}
		class:variant-filled-tertiary={$target == person && $selected != person}
		on:click={select}
	>
		{#if $settings.showAvatar}
			<header class="card-header justify-center flex">
				<Avatar src={person.avatar()} initials={person.initials()} />
			</header>
		{/if}
		{#if $settings.showNames}
			<section class="p-4">
				<p class="font-bold text-center">{person.name()}</p>
			</section>
		{/if}
	</div>
</div>

<style>
	.person-card {
		width: 200px;
		height: 200px;
		cursor: pointer;
	}
</style>
