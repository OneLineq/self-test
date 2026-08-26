// ============================================================
// 理论训练考核系统 — 首选项 Store（localStorage 持久化）
// ============================================================
import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

const STORAGE_KEY = 'self_test_preferences'

interface PreferencesData {
  indeterminateMode: boolean
  sortAnswerOrder: boolean
  shuffleOptions: boolean
}

function load(): PreferencesData {
  const defaults: PreferencesData = {
    indeterminateMode: false,
    sortAnswerOrder: true,
    shuffleOptions: false,
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<PreferencesData>
      return {
        indeterminateMode: !!parsed.indeterminateMode,
        sortAnswerOrder:
          typeof parsed.sortAnswerOrder === 'boolean'
            ? parsed.sortAnswerOrder
            : defaults.sortAnswerOrder,
        shuffleOptions: !!parsed.shuffleOptions,
      }
    }
  } catch {
    // ignore corrupt storage
  }
  return defaults
}

export const usePreferencesStore = defineStore('preferences', () => {
  const saved = load()
  const indeterminateMode = ref(saved.indeterminateMode)
  const sortAnswerOrder = ref(saved.sortAnswerOrder)
  const shuffleOptions = ref(saved.shuffleOptions)

  function persist() {
    const data: PreferencesData = {
      indeterminateMode: indeterminateMode.value,
      sortAnswerOrder: sortAnswerOrder.value,
      shuffleOptions: shuffleOptions.value,
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
  }

  watch([indeterminateMode, sortAnswerOrder, shuffleOptions], persist)

  return {
    indeterminateMode,
    sortAnswerOrder,
    shuffleOptions,
  }
})
