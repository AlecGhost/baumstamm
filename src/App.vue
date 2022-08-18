<script setup lang="ts">
import { ref } from 'vue'
import BaumstammGrid from './components/BaumstammGrid.vue'

const rows = ref(4)
const columns = ref(5)

function zoomIn() {
  columns.value--
  rows.value--
}

function zoomOut() {
  columns.value++
  rows.value++
}

</script>

<template>
  <button @click="zoomOut">+</button>
  <button @click="zoomIn">-</button>
  <BaumstammGrid :columns="columns" :rows="rows"></BaumstammGrid>
</template>

