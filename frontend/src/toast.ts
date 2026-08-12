import { reactive } from "vue";

export const toast = reactive({
  message: "",
  visible: false,
});

let timer: ReturnType<typeof setTimeout> | undefined;

export function showToast(message: string, durationMs = 2500) {
  toast.message = message;
  toast.visible = true;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => {
    toast.visible = false;
  }, durationMs);
}
