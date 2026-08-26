<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — 题库管理
// ============================================================
import { onMounted, ref, computed, h } from 'vue'
import { useRouter } from 'vue-router'
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  ExclamationCircleOutlined,
  SearchOutlined,
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { useBankStore } from '../stores/bank'
import { bankNameMatches } from '../types'

const router = useRouter()
const bankStore = useBankStore()
const createVisible = ref(false)
const renameVisible = ref(false)
const newName = ref('')
const editingBank = ref<{ id: string; name: string } | null>(null)
const bankKeyword = ref('')

const filteredBanks = computed(() =>
  bankStore.banks.filter(b => bankNameMatches(b.name, bankKeyword.value)),
)

onMounted(() => {
  bankStore.fetchBanks()
})

async function handleCreate() {
  if (!newName.value.trim()) return
  try {
    await bankStore.createBank(newName.value.trim())
    message.success('题库创建成功')
    newName.value = ''
    createVisible.value = false
  } catch (e) {
    message.error('创建失败: ' + e)
  }
}

function showRename(bank: { id: string; name: string }) {
  editingBank.value = { ...bank }
  newName.value = bank.name
  renameVisible.value = true
}

async function handleRename() {
  if (!newName.value.trim() || !editingBank.value) return
  try {
    await bankStore.renameBank(editingBank.value.id, newName.value.trim())
    message.success('重命名成功')
    renameVisible.value = false
  } catch (e) {
    message.error('重命名失败: ' + e)
  }
}

function handleDelete(bank: { id: string; name: string }) {
  Modal.confirm({
    title: '确认删除',
    icon: h(ExclamationCircleOutlined),
    content: `确定要删除题库「${bank.name}」吗？该题库下的所有题目和练习记录也将被删除，此操作不可恢复。`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await bankStore.deleteBank(bank.id)
        message.success('删除成功')
      } catch (e) {
        message.error('删除失败: ' + e)
      }
    },
  })
}

const columns = [
  { title: '题库名称', dataIndex: 'name', key: 'name' },
  { title: '题目数量', dataIndex: 'question_count', key: 'question_count', width: 100 },
  { title: '创建时间', dataIndex: 'created_at', key: 'created_at', width: 180 },
  { title: '操作', key: 'action', width: 280 },
]
</script>

<template>
  <div>
    <div style="margin-bottom: 16px; display: flex; align-items: center; gap: 12px; flex-wrap: wrap">
      <a-button type="primary" @click="createVisible = true">
        <PlusOutlined /> 新建题库
      </a-button>
      <a-input
        v-model:value="bankKeyword"
        placeholder="搜索题库名称"
        allow-clear
        style="width: 240px; margin-left: auto"
      >
        <template #prefix><SearchOutlined style="color: #bfbfbf" /></template>
      </a-input>
    </div>

    <a-table
      :columns="columns"
      :data-source="filteredBanks"
      :loading="bankStore.loading"
      row-key="id"
      :pagination="false"
    >
      <template #bodyCell="{ column, record }">
        <template v-if="column.key === 'action'">
          <a-space>
            <a-button size="small" @click="router.push(`/questions/${record.id}`)" style="white-space: nowrap">
              试题管理
            </a-button>
            <a-button size="small" @click="showRename(record)">
              <EditOutlined /> 重命名
            </a-button>
            <a-button size="small" danger @click="handleDelete(record)">
              <DeleteOutlined /> 删除
            </a-button>
          </a-space>
        </template>
      </template>
    </a-table>

    <!-- 创建对话框 -->
    <a-modal
      v-model:open="createVisible"
      title="新建题库"
      @ok="handleCreate"
    >
      <a-input
        v-model:value="newName"
        placeholder="请输入题库名称"
        @press-enter="handleCreate"
      />
    </a-modal>

    <!-- 重命名对话框 -->
    <a-modal
      v-model:open="renameVisible"
      title="重命名题库"
      @ok="handleRename"
    >
      <a-input
        v-model:value="newName"
        placeholder="请输入新名称"
        @press-enter="handleRename"
      />
    </a-modal>
  </div>
</template>



