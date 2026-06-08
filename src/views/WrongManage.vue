<script setup lang="ts">
// ============================================================
// 刷题助手 — 错题管理：查看并移出错题集
// ============================================================
import { onMounted, ref, h } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { message, Modal } from 'ant-design-vue'
import {
  ExclamationCircleOutlined,
  DeleteOutlined,
  CheckCircleOutlined,
} from '@ant-design/icons-vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { Question } from '../types'

const route = useRoute()
const router = useRouter()
const bankId = route.params.bankId as string
const bankName = route.query.name as string || ''

const wrongQuestions = ref<Question[]>([])
const loading = ref(false)

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
      <span style="color: #999; font-size: 13px">共 {{ wrongQuestions.length }} 道错题</span>
    </div>

    <a-divider />

    <a-spin :spinning="loading">
      <div v-if="wrongQuestions.length === 0" style="text-align: center; padding: 80px">
        <a-empty description="暂无错题，太棒了！">
          <a-button type="primary" @click="router.push('/')">返回首页</a-button>
        </a-empty>
      </div>

      <div v-else class="wrong-list">
        <div
          v-for="(q, idx) in wrongQuestions"
          :key="q.id"
          class="wrong-card"
        >
          <div class="wrong-card-header">
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
}
</style>
