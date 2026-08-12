<script setup lang="ts">
import { toast } from "./toast";
</script>

<template>
  <router-view />
  <div class="toast" :class="{ hidden: !toast.visible }">{{ toast.message }}</div>
</template>
