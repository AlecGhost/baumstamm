<script lang="ts">
	import { Avatar } from '@skeletonlabs/skeleton';
	import type { Person } from '$lib/Person';
	import { selected, target } from '$lib/store';
	import { imageSrc } from '$lib/image';

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
		<header class="card-header justify-center flex">
			{#await imageSrc(person.image) then src}
				<Avatar {src} initials={person.initials()} />
			{/await}
		</header>
		<section class="p-4">
			<p class="font-bold text-center">{person.name()}</p>
		</section>
	</div>
</div>

<style>
	.person-card {
		width: 200px;
		height: 200px;
		cursor: pointer;
	}
</style>
