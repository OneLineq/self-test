<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — 首页
// ============================================================
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import {
  BookOutlined,
  ThunderboltOutlined,
  BugOutlined,
  FormOutlined,
  PlusOutlined,
  RightOutlined,
} from '@ant-design/icons-vue'
import { useBankStore } from '../stores/bank'
import { PracticeModeLabel } from '../types'

const router = useRouter()
const bankStore = useBankStore()

const modes = [
  { key: 'sequential', icon: BookOutlined, color: '#1890ff', desc: '按题库顺序逐题练习' },
  { key: 'random', icon: ThunderboltOutlined, color: '#722ed1', desc: '随机打乱题目顺序' },
  { key: 'wrong', icon: BugOutlined, color: '#f5222d', desc: '只练习做错的题目' },
  { key: 'exam', icon: FormOutlined, color: '#fa8c16', desc: '限时模拟考试模式' },
]

function goPractice(bankId: string, mode: string) {
  if (mode === 'exam') {
    router.push(`/exam/${bankId}`)
  } else {
    router.push(`/practice/${bankId}?mode=${mode}`)
  }
}

function goManage(bankId: string) {
  router.push(`/questions/${bankId}`)
}

onMounted(() => {
  bankStore.fetchBanks()
})
</script>

<template>
  <div>
    <a-alert
      message="欢迎使用理论训练考核系统"
      description="选择一个题库，然后选择练习模式开始刷题。支持顺序练习、随机练习、错题练习和模拟考试。"
      type="info"
      show-icon
      closable
      style="margin-bottom: 24px"
    />

    <!-- 题库列表 -->
    <a-spin :spinning="bankStore.loading">
      <div v-if="bankStore.banks.length === 0" style="text-align: center; padding: 40px">
        <a-empty description="还没有题库">
          <a-button type="primary" @click="router.push('/banks')">
            <PlusOutlined /> 创建题库
          </a-button>
        </a-empty>
      </div>

      <div v-else>
        <a-row :gutter="[16, 16]">
          <a-col
            v-for="bank in bankStore.banks"
            :key="bank.id"
            :xs="24"
            :sm="24"
            :md="12"
            :lg="8"
          >
            <a-card :title="bank.name" size="small" hoverable>
              <template #extra>
                <a-button type="link" size="small" @click="goManage(bank.id)" style="white-space: nowrap">
                  管理 <RightOutlined />
                </a-button>
              </template>

              <a-descriptions :column="2" size="small" style="margin-bottom: 12px">
                <a-descriptions-item label="题目数">
                  {{ bank.question_count }}
                </a-descriptions-item>
                <a-descriptions-item label="创建时间">
                  {{ bank.created_at }}
                </a-descriptions-item>
              </a-descriptions>

              <!-- 练习模式按钮 -->
              <a-space wrap :size="[8, 8]">
                <a-button
                  v-for="m in modes"
                  :key="m.key"
                  size="small"
                  :style="{ borderColor: m.color, color: m.color }"
                  :disabled="bank.question_count === 0 && m.key !== 'wrong'"
                  @click="goPractice(bank.id, m.key)"
                >
                  <component :is="m.icon" />
                  {{ PracticeModeLabel[m.key as keyof typeof PracticeModeLabel] }}
                </a-button>
              </a-space>
            </a-card>
          </a-col>
        </a-row>
      </div>
    </a-spin>
  </div>
</template>
