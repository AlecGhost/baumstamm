<script lang="ts">
	import { addChild, type PersonId, type Relationship, type RelationshipId } from '../../bindings';
	import { persons, update } from '$lib/store';
	import { onDestroy, onMount } from 'svelte';
	import type { Unsubscriber } from 'svelte/motion';
	import type { Person } from '$lib/Person';

	export let ownRelationships: Relationship[];
	export let pid: PersonId;

	let personStore: Person[] = [];
	let unsubscribe: Unsubscriber;

	onMount(() => {
		unsubscribe = persons.subscribe((store) => (personStore = store));
	});

	onDestroy(() => {
		unsubscribe();
	});

	function calculateLabel(rel: Relationship): string {
		const partnerId = rel.parents.find((parent) => parent !== pid);
		if (partnerId !== null && partnerId !== undefined) {
			const partner = personStore.find((person) => person.id == partnerId)!;
			return partner.name();
		}
		return 'Unknown';
	}

	let childPartner: RelationshipId;
	function newChild() {
		addChild(childPartner);
		update();
	}
</script>

<section class="p-4">
	<span class="label">Children:</span>
	{#if ownRelationships.length != 0}
		<div class="table-container m-1">
			<table class="table table-hover">
				<tbody>
					{#each ownRelationships
						.flatMap((rel) => rel.children)
						.map((pid) => $persons.find((person) => person.id == pid)) as child}
						<tr>
							<td class="table-cell-fit">{child?.name()}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<button on:click={newChild} type="button" class="btn variant-filled m-1">Add Child with</button>
		<select bind:value={childPartner} class="select m-1">
			{#each ownRelationships as rel}
				<option value={rel.id}>{calculateLabel(rel)}</option>
			{/each}
		</select>
	{/if}
</section>
