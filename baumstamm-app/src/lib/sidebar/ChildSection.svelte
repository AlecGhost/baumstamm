<script lang="ts">
	import { addChild, type PersonId, type Relationship } from '$lib/../bindings';
	import { persons, selected, update, updateSelected, updateTarget } from '$lib/store';
	import { onDestroy, onMount } from 'svelte';
	import type { Unsubscriber } from 'svelte/motion';
	import type { Person } from '$lib/Person';

	export let ownRelationships: Relationship[];
	export let pid: PersonId;

	// clear when active person changes
	let lastPid = pid;
	$: if (pid !== lastPid) {
		relIndex = null;
		lastPid = pid;
	}

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

	let relIndex: number | null = null;

	$: {
		if (relIndex == null) {
			updateTarget(null);
		} else if (ownRelationships !== undefined && ownRelationships.length > 0) {
			const partnerId = ownRelationships[relIndex].parents.find((parent) => parent !== pid);
			if (partnerId !== null && partnerId !== undefined) {
				updateTarget(partnerId);
			}
		}
	}

	async function newChild() {
		if (relIndex !== null) {
			let pid = await addChild(ownRelationships[relIndex].id);
			await update();
			updateSelected(pid);
		}
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
						<tr on:click={() => ($selected = child ?? null)} class="cursor-pointer">
							<td class="table-cell-fit">{child?.name()}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<button on:click={newChild} type="button" class="btn variant-filled m-1">Add Child with</button>
		<select bind:value={relIndex} class="select m-1">
			{#each ownRelationships as rel, i}
				<option value={i}>{calculateLabel(rel)}</option>
			{/each}
		</select>
	{/if}
</section>
