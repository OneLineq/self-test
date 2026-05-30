// ============================================================
// 刷题助手 — 题库 Store (Pinia)
// ============================================================
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { Bank } from '../types'

export const useBankStore = defineStore('bank', () => {
  const banks = ref<Bank[]>([])
  const currentBank = ref<Bank | null>(null)
  const loading = ref(false)

  /** 获取所有题库 */
  async function fetchBanks() {
    loading.value = true
    try {
      banks.value = await invoke<Bank[]>('list_banks')
    } finally {
      loading.value = false
    }
  }

  /** 创建题库 */
  async function createBank(name: string): Promise<Bank> {
    const bank = await invoke<Bank>('create_bank', { name })
    await fetchBanks()
    return bank
  }

  /** 删除题库 */
  async function deleteBank(id: string) {
    await invoke('delete_bank', { id })
    await fetchBanks()
  }

  /** 重命名题库 */
  async function renameBank(id: string, name: string) {
    await invoke('rename_bank', { id, name })
    await fetchBanks()
  }

  /** 加载单个题库 */
  async function loadBank(id: string): Promise<Bank | null> {
    const bank = await invoke<Bank | null>('get_bank', { id })
    currentBank.value = bank
    return bank
  }

  return {
    banks,
    currentBank,
    loading,
    fetchBanks,
    createBank,
    deleteBank,
    renameBank,
    loadBank,
  }
})
