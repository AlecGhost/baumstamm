<script lang="ts">
	import { Avatar } from '@skeletonlabs/skeleton';
	import type { Person } from '$lib/Person';

	export let person: Person;
</script>

<div class="person-card card p-4">
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
	}
</style>
