<script lang="ts">
	import { Avatar } from '@skeletonlabs/skeleton';
	import type { Person } from '$lib/Person';
	import { selected } from './store';

	export let person: Person;

	function select() {
		selected.update(() => person);
	}
</script>

<div class="person-card card p-4" on:click={select}>
	<header class="card-header justify-center flex">
		<Avatar src={person.avatar()} initials={person.initials()} />
	</header>
	<section class="p-4">
		<p class="font-bold text-center">{person.name()}</p>
	</section>
</div>

<style>
	.person-card {
		width: 200px;
		/* user-select: all; */
		cursor: pointer;
	}
</style>
