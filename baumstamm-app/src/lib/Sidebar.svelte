<script lang="ts">
	import { Avatar } from '@skeletonlabs/skeleton';
	import type { Person } from './Person';
	import type { Relationship } from '../bindings';
	import { relationships } from './store';
	import { onDestroy, onMount } from 'svelte';
	import type { Unsubscriber } from 'svelte/store';
	import InfoSection from './sidebar/InfoSection.svelte';
	import ParentSection from './sidebar/ParentSection.svelte';
	import ChildSection from './sidebar/ChildSection.svelte';
	import PartnerSection from './sidebar/PartnerSection.svelte';

	export let person: Person | null;

	// stores
	let unsubscribe: Unsubscriber;
	let relationshipStore: Relationship[] = [];

	onMount(() => {
		unsubscribe = relationships.subscribe((store) => (relationshipStore = store));
	});

	onDestroy(() => {
		unsubscribe();
	});

	let ownRelationships: Relationship[] = [];
	$: {
		if (person !== null) {
			ownRelationships = relationshipStore.filter((rel) => rel.parents.includes(person!.id));
		}
	}
	let parentRel: Relationship;
	$: {
		if (person !== null) {
			parentRel = relationshipStore.find((rel) => rel.children.includes(person!.id))!;
		}
	}
</script>

<div class="p-4">
	{#if person !== null}
		<section class="flex justify-center p-4">
			<Avatar src={person.avatar()} initials={person.initials()} width="w-60" />
		</section>
		<p class="font-bold text-center">{person.name()}</p>
		<InfoSection {person} />
		<hr />
		{#if parentRel !== undefined}
			<ParentSection {parentRel} />
		{/if}
		<hr />
		<ChildSection {ownRelationships} pid={person.id} />
		<hr />
		<PartnerSection {ownRelationships} pid={person.id} />
	{:else}
		<div class="flex justify-center p-4">
			<div class="placeholder-circle animate-pulse w-60" />
		</div>
		<p class="text-center">No person selected.</p>
	{/if}
</div>
