<script setup lang="ts">
import ConnectorType from '../types/ConnectorType'
import { GridItemType, GridItemWrapper } from '../types/GridItemWrapper'
import GridItem from './GridItem.vue'

defineProps<{
  rows: number,
  columns: number
}>()

const gridItem = new GridItemWrapper(GridItemType.Connector, ConnectorType.T)
</script>

<template>
  <section class="grid-container">
    <div class="grid-item" v-for="n in rows * columns">
      <GridItem :gridItem="gridItem" />
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
