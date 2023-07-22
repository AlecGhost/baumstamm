<script lang="ts">
	import { Avatar } from '@skeletonlabs/skeleton';
	import type { Person } from '$lib/Person';
	import { selected } from './store';

	export let person: Person;

	function select() {
		selected.update(() => person);
	}
</script>

<div class="person-card m-0">
	<div class="card p-4 mx-4 h-full" class:border={$selected == person} on:click={select}>
		<header class="card-header justify-center flex">
			<Avatar src={person.avatar()} initials={person.initials()} />
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
