<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — 错题管理：查看并移出错题集
// ============================================================
import { onMounted, ref, computed, h } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { message, Modal } from 'ant-design-vue'
import {
  ExclamationCircleOutlined,
  DeleteOutlined,
  CheckCircleOutlined,
  SearchOutlined,
} from '@ant-design/icons-vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { Question } from '../types'
import { questionTextMatches } from '../types'

const route = useRoute()
const router = useRouter()
const bankId = route.params.bankId as string
const bankName = route.query.name as string || ''

const wrongQuestions = ref<Question[]>([])
const loading = ref(false)
const selectedIds = ref<Set<string>>(new Set())
const questionKeyword = ref('')

const filteredWrongQuestions = computed(() =>
  wrongQuestions.value.filter(q => questionTextMatches(q, questionKeyword.value)),
)

const isAllSelected = computed(() =>
  filteredWrongQuestions.value.length > 0
  && filteredWrongQuestions.value.every(q => selectedIds.value.has(q.id)),
)

const selectedInFilteredCount = computed(() =>
  filteredWrongQuestions.value.filter(q => selectedIds.value.has(q.id)).length,
)

function toggleSelect(id: string) {
  const s = new Set(selectedIds.value)
  if (s.has(id)) {
    s.delete(id)
  } else {
    s.add(id)
  }
  selectedIds.value = s
}

function toggleSelectAll() {
  if (isAllSelected.value) {
    const filteredIds = new Set(filteredWrongQuestions.value.map(q => q.id))
    selectedIds.value = new Set([...selectedIds.value].filter(id => !filteredIds.has(id)))
  } else {
    const s = new Set(selectedIds.value)
    for (const q of filteredWrongQuestions.value) {
      s.add(q.id)
    }
    selectedIds.value = s
  }
}

onMounted(async () => {
  await loadWrongQuestions()
})

async function loadWrongQuestions() {
  loading.value = true
  try {
    wrongQuestions.value = await invoke<Question[]>('get_practice_questions', {
      bankId,
      mode: 'wrong',
      questionTypes: null as any,
      limit: null as any,
      perTypeLimits: null as any,
    })
  } catch (e) {
    message.error('加载错题失败: ' + e)
  } finally {
    loading.value = false
  }
}

/** 批量移出错题集 */
async function handleBatchRemove() {
  if (selectedIds.value.size === 0) return
  Modal.confirm({
    title: '批量移出错题集',
    icon: h(ExclamationCircleOutlined),
    content: `确定将选中的 ${selectedIds.value.size} 道题移出错题集吗？`,
    okText: '批量移出',
    cancelText: '取消',
    async onOk() {
      try {
        const count = await invoke<number>('batch_remove_from_wrong', {
          questionIds: Array.from(selectedIds.value),
        })
        message.success(`已移出 ${count} 道错题`)
        wrongQuestions.value = wrongQuestions.value.filter(wq => !selectedIds.value.has(wq.id))
        selectedIds.value = new Set()
      } catch (e) {
        message.error('批量移出失败: ' + e)
      }
    },
  })
}

/** 移出错题集 */
async function removeFromWrong(q: Question) {
  Modal.confirm({
    title: '移出错题集',
    icon: h(ExclamationCircleOutlined),
    content: `确定将本题「${q.stem.slice(0, 50)}${q.stem.length > 50 ? '...' : ''}」移出错题集吗？`,
    okText: '移出',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('remove_from_wrong', { questionId: q.id })
        message.success('已移出错题集')
        // 从列表中移除该题
        wrongQuestions.value = wrongQuestions.value.filter(wq => wq.id !== q.id)
      } catch (e) {
        message.error('移出失败: ' + e)
      }
    },
  })
}

/** 解析答案文字 */
function formatAnswerText(q: Question, answer: string | string[]): string {
  const ans = Array.isArray(answer) ? answer.join(',') : answer
  if (!ans) return '(空)'
  const parts = ans.split(',').filter(Boolean)
  return parts.map(p => {
    const idx = p.charCodeAt(0) - 65
    if (idx >= 0 && idx < q.options.length) {
      return `${p}. ${q.options[idx].replace(/^[A-D][.、]\s*/, '')}`
    }
    return p
  }).join('；')
}
</script>

<template>
  <div class="wrong-manage-page">
    <div class="page-header">
      <a-space>
        <a-button @click="router.back()">← 返回</a-button>
        <h2 style="margin: 0">错题管理</h2>
        <span v-if="bankName" style="color: #999; font-size: 14px">— {{ bankName }}</span>
      </a-space>
      <a-space>
        <a-input
          v-if="wrongQuestions.length > 0"
          v-model:value="questionKeyword"
          placeholder="搜索题干或选项"
          allow-clear
          style="width: 220px"
        >
          <template #prefix><SearchOutlined style="color: #bfbfbf" /></template>
        </a-input>
        <span style="color: #999; font-size: 13px">
          共 {{ filteredWrongQuestions.length }} 道错题
          <template v-if="filteredWrongQuestions.length !== wrongQuestions.length">
            （筛选自 {{ wrongQuestions.length }} 题）
          </template>
        </span>
        <a-button
          v-if="wrongQuestions.length > 0"
          size="small"
          danger
          :disabled="selectedIds.size === 0"
          @click="handleBatchRemove"
        >
          <DeleteOutlined /> 批量移出错题集
        </a-button>
      </a-space>
    </div>

    <a-divider />

    <a-spin :spinning="loading">
      <div v-if="wrongQuestions.length === 0" style="text-align: center; padding: 80px">
        <a-empty description="暂无错题，太棒了！">
          <a-button type="primary" @click="router.push('/')">返回首页</a-button>
        </a-empty>
      </div>

      <div v-else>
        <div v-if="filteredWrongQuestions.length === 0" style="text-align: center; padding: 40px">
          <a-empty description="未找到匹配错题" />
        </div>
        <template v-else>
        <!-- 批量操作工具栏 -->
        <div class="batch-toolbar">
          <a-checkbox
            :checked="isAllSelected"
            :indeterminate="selectedInFilteredCount > 0 && !isAllSelected"
            @change="toggleSelectAll"
          >
            全选
          </a-checkbox>
          <span style="color: #999; font-size: 13px; margin-left: 8px">
            已选 {{ selectedIds.size }} 题
          </span>
        </div>

        <div class="wrong-list">
          <div
            v-for="(q, idx) in filteredWrongQuestions"
            :key="q.id"
            class="wrong-card"
            :class="{ 'wrong-card-selected': selectedIds.has(q.id) }"
          >
            <div class="wrong-card-header">
              <a-checkbox
                :checked="selectedIds.has(q.id)"
                @change="toggleSelect(q.id)"
                style="margin-right: 4px"
              />
              <span class="wrong-q-number">{{ idx + 1 }}.</span>
              <a-tag v-if="q.type" size="small" color="orange">{{ q.type === 'single' ? '单选' : q.type === 'multiple' ? '多选' : q.type === 'judge' ? '判断' : q.type === 'fill' ? '填空' : q.type }}</a-tag>
              <span class="wrong-stem">{{ q.stem }}</span>
              <a-button
                size="small"
                danger
                @click="removeFromWrong(q)"
              >
                <DeleteOutlined /> 移出错题集
              </a-button>
            </div>

          <!-- 选项 -->
          <div v-if="q.options.length > 0" class="wrong-options">
            <div
              v-for="(opt, oi) in q.options"
              :key="oi"
              class="wrong-option"
            >
              <span class="option-letter">{{ String.fromCharCode(65 + oi) }}</span>
              <span class="option-text">{{ opt.replace(/^[A-D][.、]\s*/, '') }}</span>
            </div>
          </div>

          <!-- 正确答案 -->
          <div class="wrong-answer">
            <CheckCircleOutlined style="color: #52c41a; margin-right: 4px" />
            <span style="color: #999">正确答案：</span>
            <span style="color: #52c41a; font-weight: 500">{{ formatAnswerText(q, q.answer) }}</span>
          </div>

          <!-- 解析 -->
          <div v-if="q.explanation" class="wrong-explanation">
            <span style="color: #1890ff">💡 解析：</span>
            {{ q.explanation }}
          </div>
          </div>
        </div>
        </template>
      </div>
    </a-spin>
  </div>
</template>

<style scoped>
.wrong-manage-page {
  max-width: 900px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.wrong-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.wrong-card {
  border: 1px solid #e8e8e8;
  border-radius: 8px;
  padding: 16px;
  background: #fff;
  border-left: 4px solid #f5222d;
  transition: box-shadow 0.2s;
}

.wrong-card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.wrong-card-selected {
  background: #fff2f0;
  border-color: #ffa39e;
}

.batch-toolbar {
  display: flex;
  align-items: center;
  padding: 8px 4px;
  margin-bottom: 12px;
}

.wrong-card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.wrong-q-number {
  font-weight: bold;
  color: #f5222d;
}

.wrong-stem {
  flex: 1;
  font-size: 15px;
  white-space: pre-line;
}

.wrong-options {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 8px 0 8px 24px;
}

.wrong-option {
  display: flex;
  align-items: center;
  padding: 6px 10px;
  border: 1px solid #e8e8e8;
  border-radius: 6px;
  font-size: 13px;
}

.wrong-option .option-letter {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: #f0f0f0;
  font-weight: bold;
  margin-right: 10px;
  flex-shrink: 0;
  font-size: 12px;
}

.wrong-option .option-text {
  flex: 1;
}

.wrong-answer {
  margin-top: 8px;
  padding: 8px 12px;
  background: #f6ffed;
  border-radius: 6px;
  font-size: 13px;
}

.wrong-explanation {
  margin-top: 8px;
  padding: 8px 12px;
  background: #e6f7ff;
  border-radius: 6px;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-line;
}
</style>
