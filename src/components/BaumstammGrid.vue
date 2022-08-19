<script setup lang="ts">
import { GridItemWrapper } from '../types/GridItemWrapper'
import GridItem from './GridItem.vue'

defineProps<{
  rows: number,
  columns: number,
  gridItems: GridItemWrapper[]
}>()

</script>

<template>
  <section class="grid-container">
    <div class="grid-item" v-if="rows * columns === gridItems.length" v-for="gridItem in gridItems">
      <GridItem :gridItem="gridItem"/>
    </div>
  </section>
</template>

<style scoped>
.grid-container {
  display: grid;
  grid-template-columns: repeat(v-bind(columns), 1fr);
  height: 90vh;
}

.grid-item {
  background-color: green;
  border-style: solid;
  border-color: blue;
}
</style>
