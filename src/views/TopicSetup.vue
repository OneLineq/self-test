<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — 主题练习设置（关键词筛选 + 选择练习方式）
// ============================================================
import { onMounted, ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { message } from 'ant-design-vue'
import {
  SearchOutlined,
  BookOutlined,
  ThunderboltOutlined,
  BugOutlined,
  LeftOutlined,
} from '@ant-design/icons-vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { Question } from '../types'
import {
  normalizeQuestion,
  compileTopicMatchers,
  questionMatchesTopic,
  saveTopicFilter,
} from '../types'

const route = useRoute()
const router = useRouter()
const bankId = route.params.bankId as string

const loading = ref(true)
const bankName = ref('')
const allQuestions = ref<Question[]>([])
const wrongIds = ref<Set<string>>(new Set())

const keywordRaw = ref('')
const useRegex = ref(false)
const matchMode = ref<'any' | 'all'>('any')
const practiceMode = ref<'sequential' | 'random' | 'wrong'>('sequential')

const regexError = computed(() => {
  if (!keywordRaw.value.trim()) return ''
  const compiled = compileTopicMatchers(keywordRaw.value, useRegex.value)
  return compiled.ok ? '' : compiled.error
})

const filteredQuestions = computed(() => {
  if (!keywordRaw.value.trim() || regexError.value) return []
  let list = allQuestions.value
  if (practiceMode.value === 'wrong') {
    list = list.filter(q => wrongIds.value.has(q.id))
  }
  return list.filter(q =>
    questionMatchesTopic(q, keywordRaw.value, useRegex.value, matchMode.value),
  )
})

const hitCount = computed(() => filteredQuestions.value.length)

const canStart = computed(() =>
  !!keywordRaw.value.trim() && !regexError.value && hitCount.value > 0,
)

onMounted(async () => {
  try {
    const bank = await invoke<{ name: string } | null>('get_bank', { id: bankId })
    bankName.value = bank?.name || ''
    const raw = await invoke<Question[]>('get_practice_questions', {
      bankId,
      mode: 'sequential',
      questionTypes: null,
      limit: null,
      perTypeLimits: null,
    })
    allQuestions.value = raw.map(normalizeQuestion)
    try {
      const ids = await invoke<string[]>('list_wrong_question_ids', { bankId })
      wrongIds.value = new Set(ids)
    } catch {
      wrongIds.value = new Set()
    }
  } catch (e) {
    message.error('加载题目失败: ' + e)
    router.back()
  } finally {
    loading.value = false
  }
})

function startPractice() {
  if (!canStart.value) {
    if (regexError.value) message.warning(regexError.value)
    else if (!keywordRaw.value.trim()) message.warning('请输入关键词')
    else message.warning('没有匹配的题目')
    return
  }
  saveTopicFilter({
    bankId,
    raw: keywordRaw.value,
    regex: useRegex.value,
    match: matchMode.value,
  })
  router.push(`/practice/${bankId}?mode=${practiceMode.value}&topic=1`)
}

const modeOptions: Array<{
  value: 'sequential' | 'random' | 'wrong'
  label: string
  icon: typeof BookOutlined
  color: string
  desc: string
}> = [
  { value: 'sequential', label: '顺序练习', icon: BookOutlined, color: '#1890ff', desc: '按题库顺序练习匹配题' },
  { value: 'random', label: '随机练习', icon: ThunderboltOutlined, color: '#722ed1', desc: '打乱匹配题顺序' },
  { value: 'wrong', label: '错题练习', icon: BugOutlined, color: '#f5222d', desc: '仅在错题集中匹配' },
]
</script>

<template>
  <div class="topic-setup">
    <div style="margin-bottom: 16px; display: flex; align-items: center; gap: 12px">
      <a-button @click="router.push('/topic')"><LeftOutlined /> 返回</a-button>
      <h2 style="margin: 0">主题练习</h2>
      <a-tag v-if="bankName" color="blue">{{ bankName }}</a-tag>
    </div>

    <a-spin :spinning="loading">
      <a-card title="筛选条件" style="margin-bottom: 16px">
        <a-form layout="vertical">
          <a-form-item label="关键词">
            <a-textarea
              v-model:value="keywordRaw"
              :rows="3"
              placeholder="输入关键词，多个用空格或逗号分隔。开启正则后，每个词作为一条正则表达式。"
              allow-clear
            />
            <div style="margin-top: 8px; color: #999; font-size: 12px">
              匹配范围：题干与选项。示例：网络 路由　或　网络,路由
            </div>
          </a-form-item>

          <a-form-item label="匹配方式">
            <a-radio-group v-model:value="matchMode" button-style="solid">
              <a-radio-button value="any">任一命中</a-radio-button>
              <a-radio-button value="all">全部命中</a-radio-button>
            </a-radio-group>
            <a-switch v-model:checked="useRegex" style="margin-left: 16px" />
            <span style="margin-left: 8px; color: #666">正则表达式</span>
          </a-form-item>

          <a-alert
            v-if="regexError"
            type="error"
            show-icon
            :message="regexError"
            style="margin-bottom: 12px"
          />
        </a-form>
      </a-card>

      <a-card title="练习方式" style="margin-bottom: 16px">
        <a-radio-group v-model:value="practiceMode" style="width: 100%">
          <a-row :gutter="[12, 12]">
            <a-col v-for="m in modeOptions" :key="m.value" :xs="24" :sm="8">
              <div
                class="mode-option"
                :class="{ active: practiceMode === m.value }"
                :style="{ borderColor: practiceMode === m.value ? m.color : '#e8e8e8' }"
                @click="practiceMode = m.value"
              >
                <a-radio :value="m.value">
                  <component :is="m.icon" :style="{ color: m.color, marginRight: '6px' }" />
                  {{ m.label }}
                </a-radio>
                <div style="font-size: 12px; color: #999; margin-top: 4px; padding-left: 24px">
                  {{ m.desc }}
                </div>
              </div>
            </a-col>
          </a-row>
        </a-radio-group>
      </a-card>

      <div style="text-align: center; padding: 8px 0 24px">
        <div style="margin-bottom: 12px; color: #666">
          当前命中
          <strong :style="{ color: hitCount > 0 ? '#1890ff' : '#f5222d', fontSize: '18px' }">
            {{ hitCount }}
          </strong>
          题
          <template v-if="practiceMode === 'wrong'">
            （错题集共 {{ wrongIds.size }} 题）
          </template>
          <template v-else>
            （题库共 {{ allQuestions.length }} 题）
          </template>
        </div>
        <a-button type="primary" size="large" :disabled="!canStart" @click="startPractice">
          <SearchOutlined /> 开始练习（{{ hitCount }} 题）
        </a-button>
      </div>
    </a-spin>
  </div>
</template>

<style scoped>
.topic-setup {
  max-width: 720px;
  margin: 0 auto;
}

.mode-option {
  border: 1px solid #e8e8e8;
  border-radius: 8px;
  padding: 12px;
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s;
  height: 100%;
}

.mode-option:hover {
  background: #fafafa;
}

.mode-option.active {
  background: #f0f7ff;
}
</style>
