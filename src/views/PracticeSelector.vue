<script setup lang="ts">
// ============================================================
// 刷题助手 — 练习模式题库选择页
// 选择某个题库后跳转到对应的练习或考试页面
// ============================================================
import { onMounted, ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useBankStore } from '../stores/bank'
import { PracticeModeLabel } from '../types'
import type { PracticeMode } from '../types'
import {
  BookOutlined,
  ThunderboltOutlined,
  BugOutlined,
  FormOutlined,
  PlayCircleOutlined,
} from '@ant-design/icons-vue'

const route = useRoute()
const router = useRouter()
const bankStore = useBankStore()

const mode = computed<PracticeMode>(() => {
  const m = route.path.replace('/', '') as PracticeMode
  return ['sequential', 'random', 'wrong', 'exam'].includes(m) ? m : 'sequential'
})

// 模式对应的配置
const modeConfig = computed(() => {
  const map: Record<string, { icon: any; color: string; bg: string; desc: string }> = {
    sequential: { icon: BookOutlined, color: '#1890ff', bg: '#e6f7ff', desc: '按题库顺序逐题练习，适合系统学习' },
    random: { icon: ThunderboltOutlined, color: '#722ed1', bg: '#f9f0ff', desc: '随机打乱题目顺序，适合检验掌握程度' },
    wrong: { icon: BugOutlined, color: '#f5222d', bg: '#fff2f0', desc: '只练习曾经做错的题目，针对薄弱环节' },
    exam: { icon: FormOutlined, color: '#fa8c16', bg: '#fff7e6', desc: '限时模拟考试，还原真实考场体验' },
  }
  return map[mode.value] || map.sequential
})

const title = computed(() => PracticeModeLabel[mode.value])
const loading = ref(false)

onMounted(async () => {
  loading.value = true
  try {
    await bankStore.fetchBanks()
  } finally {
    loading.value = false
  }
})

function startPractice(bankId: string) {
  if (mode.value === 'exam') {
    router.push(`/exam/${bankId}`)
  } else {
    router.push(`/practice/${bankId}?mode=${mode.value}`)
  }
}
</script>

<template>
  <div class="selector-page">
    <!-- 模式标题 -->
    <a-card
      :style="{
        marginBottom: '24px',
        borderLeft: `4px solid ${modeConfig.color}`,
        background: modeConfig.bg,
      }"
      :body-style="{ padding: '16px 24px' }"
    >
      <a-space>
        <component :is="modeConfig.icon" :style="{ fontSize: '28px', color: modeConfig.color }" />
        <div>
          <div style="font-size: 18px; font-weight: bold; color: modeConfig.color">
            {{ title }}
          </div>
          <div style="font-size: 13px; color: #666; margin-top: 2px">
            {{ modeConfig.desc }}
          </div>
        </div>
      </a-space>
    </a-card>

    <!-- 题库选择 -->
    <a-spin :spinning="loading || bankStore.loading">
      <div v-if="bankStore.banks.length === 0" style="text-align: center; padding: 80px">
        <a-empty :description="`还没有题库，请先创建或导入题库`">
          <a-button type="primary" @click="router.push('/banks')">
            去题库管理 →
          </a-button>
        </a-empty>
      </div>

      <a-list
        v-else
        :data-source="bankStore.banks"
        :pagination="{
          pageSize: 8,
          showSizeChanger: false,
          hideOnSinglePage: true,
        }"
      >
        <template #renderItem="{ item: bank }">
          <a-list-item>
            <a-list-item-meta>
              <template #title>
                <span style="font-weight: 600; font-size: 15px">{{ bank.name }}</span>
              </template>
              <template #description>
                <a-space>
                  <span>{{ bank.question_count }} 道题目</span>
                  <span style="color: #999">|</span>
                  <span>创建于 {{ bank.created_at }}</span>
                </a-space>
              </template>
            </a-list-item-meta>

            <template #actions>
              <a-button
                type="primary"
                :style="{ background: modeConfig.color, borderColor: modeConfig.color }"
                :disabled="bank.question_count === 0 && mode !== 'wrong'"
                @click="startPractice(bank.id)"
              >
                <template #icon><PlayCircleOutlined /></template>
                开始{{ title }}
              </a-button>
              <a-button @click="router.push(mode === 'wrong' ? `/wrong-manage/${bank.id}` : `/questions/${bank.id}`)">
                {{ mode === 'wrong' ? '管理错题' : '管理题目' }}
              </a-button>
            </template>
          </a-list-item>
        </template>
      </a-list>
    </a-spin>

    <!-- 底部提示 -->
    <div v-if="bankStore.banks.length > 0" style="text-align: center; margin-top: 24px; color: #999; font-size: 12px">
      选择一个题库，点击「开始{{ title }}」进入练习
    </div>
  </div>
</template>

<style scoped>
.selector-page {
  max-width: 800px;
  margin: 0 auto;
}
</style>
