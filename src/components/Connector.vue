<script setup lang="ts">
import { defineProps, ref } from 'vue';
import ConnectorType from '../types/ConnectorType'

const props = defineProps<{
  connectorType: ConnectorType 
}>();
const leftDivTop = ref(!(props.connectorType == ConnectorType.RightCorner))
const rightDivTop = ref(!(props.connectorType == ConnectorType.LeftCorner))
const leftDivSide = ref((props.connectorType == ConnectorType.RightCorner) || (props.connectorType == ConnectorType.T))
const rightDivSide = ref(props.connectorType == ConnectorType.LeftCorner)

</script>

<template>
  <section class="connector-grid-container">
    <div></div>
    <div></div>
    <div :class="{ 'top-line': leftDivTop, 'right-line': leftDivSide }"></div>
    <div :class="{ 'top-line': rightDivTop, 'left-line': rightDivSide }"></div>
  </section>
</template>

<style scoped>
.connector-grid-container {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  height: 100%;
}

.top-line {
  border-top: solid;
  border-color: black;
}

.right-line {
  border-right: solid;
  border-color: black;
}

.left-line {
  border-left: solid;
  border-color: black;
}
</style>
