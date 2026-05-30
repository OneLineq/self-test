<script setup lang="ts">
// ============================================================
// 刷题助手 — 练习页面（顺序/随机/错题）
// ============================================================
import { onMounted, ref, computed, watch, h } from 'vue'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import { message, Modal } from 'ant-design-vue'
import {
  LeftOutlined,
  RightOutlined,
  CheckOutlined,
  CloseOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons-vue'
import { usePracticeStore } from '../stores/practice'
import { PracticeModeLabel } from '../types'
import type { PracticeMode } from '../types'

const route = useRoute()
const router = useRouter()
const store = usePracticeStore()

const bankId = route.params.bankId as string
const mode = (route.query.mode as PracticeMode) || 'sequential'
const selectedAnswer = ref('')
const jumpInput = ref<number>(1)

onMounted(async () => {
  try {
    await store.loadQuestions(bankId, mode)
  } catch (e) {
    message.error('加载题目失败')
    router.back()
  }
})

// 离页确认：防止侧边栏误触退出
// 如果侧边栏已弹过确认框（skipLeaveConfirm），则直接放行
onBeforeRouteLeave((_to, _from, next) => {
  if (store.skipLeaveConfirm) {
    store.skipLeaveConfirm = false
    next()
    return
  }
  if (store.answeredCount > 0) {
    Modal.confirm({
      title: '退出练习？',
      icon: h(ExclamationCircleOutlined),
      content: `已作答 ${store.answeredCount} / ${store.totalCount} 题，退出不会丢失已保存的记录。`,
      okText: '退出',
      cancelText: '继续答题',
      onOk: () => next(),
      onCancel: () => next(false),
    })
  } else {
    next()
  }
})

const question = computed(() => store.currentQuestion)

function selectOption(optIndex: number) {
  if (!question.value) return
  const qid = question.value.id
  if (store.showResult.get(qid)) return // already answered

  const letter = String.fromCharCode(65 + optIndex)
  const isMulti = question.value ? Array.isArray(question.value.answer) : false

  if (isMulti) {
    // 多选题：切换选中
    let current = selectedAnswer.value || ''
    const parts = current ? current.split(',').filter(Boolean) : []
    if (parts.includes(letter)) {
      selectedAnswer.value = parts.filter(p => p !== letter).join(',')
    } else {
      parts.push(letter)
      selectedAnswer.value = parts.join(',')
    }
  } else {
    // 单选题：替换
    selectedAnswer.value = letter
    // 单选题直接提交
    store.submitAnswer(qid, letter)
  }
}

function getOptionClass(optIndex: number): string {
  if (!question.value) return ''
  const qid = question.value.id
  const letter = String.fromCharCode(65 + optIndex)

  // 已提交后的结果展示：用 store 中的答案
  if (store.showResult.get(qid)) {
    const isCorrect = store.isOptionCorrect(question.value, optIndex)
    const saved = store.userAnswers.get(qid) || ''
    const isSelected = saved === letter || saved.split(',').filter(Boolean).includes(letter)
    if (isCorrect) return 'option-correct'
    if (isSelected && !isCorrect) return 'option-wrong'
    return ''
  }

  // 未提交时：展示当前本地选中状态
  const local = selectedAnswer.value || ''
  const isLocalSelected = local === letter || local.split(',').filter(Boolean).includes(letter)
  return isLocalSelected ? 'option-selected' : ''
}

// 多选提交
function submitMulti() {
  if (!question.value || !selectedAnswer.value) return
  store.submitAnswer(question.value.id, selectedAnswer.value)
}

function handleNext() {
  selectedAnswer.value = ''
  store.next()
}

function handlePrev() {
  selectedAnswer.value = ''
  store.prev()
}

// Watch for question changes to reset selection
watch(() => store.currentIndex, () => {
  const q = store.currentQuestion
  if (q) {
    selectedAnswer.value = store.userAnswers.get(q.id) || ''
  }
})

function handleJump() {
  const idx = jumpInput.value - 1
  if (idx >= 0 && idx < store.totalCount) {
    selectedAnswer.value = store.userAnswers.get(store.questions[idx]?.id) || ''
    store.goTo(idx)
  }
}

function exitPractice() {
  Modal.confirm({
    title: '退出练习',
    icon: h(ExclamationCircleOutlined),
    content: '确定要退出当前练习吗？已作答的题目记录不会丢失。',
    okText: '退出',
    cancelText: '继续练习',
    onOk: () => router.push('/'),
  })
}

const progress = computed(() =>
  store.totalCount > 0 ? Math.round((store.currentIndex / store.totalCount) * 100) : 0
)

// 题号按钮颜色：正确绿、错误红、未答灰
function navDotStyle(idx: number): Record<string, string> {
  const q = store.questions[idx]
  if (!q || !store.showResult.get(q.id)) return {}
  const correct = store.isAnswerCorrect(q.id)
  return {
    background: correct ? '#52c41a' : '#f5222d',
    color: '#fff',
    borderColor: correct ? '#52c41a' : '#f5222d',
  }
}
</script>

<template>
  <div class="practice-page">
    <!-- 顶部信息栏 -->
    <div class="practice-header">
      <a-space>
        <a-tag color="blue">{{ PracticeModeLabel[mode] }}</a-tag>
        <span>{{ store.currentIndex + 1 }} / {{ store.totalCount }}</span>
        <span style="color: #52c41a">✓ {{ store.correctCount }}</span>
      </a-space>
      <a-progress :percent="progress" :show-info="false" style="flex: 1; margin: 0 16px" />
      <a-button @click="exitPractice">退出</a-button>
    </div>

    <a-divider />

    <!-- 题目区域 -->
    <div v-if="store.loading" style="text-align: center; padding: 80px">
      <a-spin size="large" />
    </div>

    <div v-else-if="!question" style="text-align: center; padding: 80px">
      <a-empty :description="mode === 'wrong' ? '暂无错题，太棒了！' : '题库为空'">
        <a-button type="primary" @click="router.push('/')">返回首页</a-button>
      </a-empty>
    </div>

    <div v-else class="question-area">
      <!-- 题型标签 -->
      <a-tag v-if="question.type" style="margin-bottom: 12px">
        {{ question.type }}
      </a-tag>

      <!-- 题干 -->
      <div class="question-stem">
        <span class="q-number">{{ store.currentIndex + 1 }}.</span>
        {{ question.stem }}
      </div>

      <!-- 历史统计 -->
      <div v-if="question.times_attempted > 0" class="question-stats">
        历史统计：出现 {{ question.times_attempted }} 次，
        正确 {{ question.times_correct }} 次，
        正确率 {{ question.times_attempted > 0 ? Math.round(question.times_correct / question.times_attempted * 100) : 0 }}%
      </div>

      <!-- 选项 -->
      <div class="options-list">
        <div
          v-for="(opt, idx) in question.options"
          :key="idx"
          class="option-item"
          :class="getOptionClass(idx)"
          @click="selectOption(idx)"
        >
          <span class="option-letter">{{ String.fromCharCode(65 + idx) }}</span>
          <span class="option-text">{{ opt.replace(/^[A-D][.、]\s*/, '') }}</span>
          <CheckOutlined v-if="getOptionClass(idx) === 'option-correct'" class="option-icon" />
          <CloseOutlined v-if="getOptionClass(idx) === 'option-wrong'" class="option-icon" />
        </div>
      </div>

      <!-- 多选提交按钮 -->
      <div
        v-if="
          !store.showResult.get(question.id) &&
          Array.isArray(question.answer) &&
          selectedAnswer
        "
        style="text-align: center; margin-top: 16px"
      >
        <a-button type="primary" size="large" @click="submitMulti">
          确认答案（已选 {{ selectedAnswer.split(',').filter(Boolean).length }} 项）
        </a-button>
      </div>

      <!-- 解析 -->
      <a-alert
        v-if="question.explanation && store.showResult.get(question.id)"
        message="解析"
        :description="question.explanation"
        type="info"
        show-icon
        style="margin-top: 16px"
      />

      <!-- 底部导航 -->
      <div class="practice-footer">
        <a-button :disabled="store.currentIndex === 0" @click="handlePrev">
          <LeftOutlined /> 上一题
        </a-button>
        <span style="color: #999; font-size: 13px">
          {{ store.currentIndex + 1 }} / {{ store.totalCount }}
        </span>
        <a-button
          type="primary"
          :disabled="store.currentIndex >= store.totalCount - 1"
          @click="handleNext"
        >
          下一题 <RightOutlined />
        </a-button>
      </div>

      <!-- 缩略题号导航（仅显示当前附近 + 首尾） -->
      <div class="question-nav-strip">
        <a-button
          v-if="store.currentIndex > 2"
          size="small"
          class="nav-dot"
          :style="navDotStyle(0)"
          @click="store.goTo(0)"
        >1</a-button>
        <span v-if="store.currentIndex > 3" class="nav-ellipsis">…</span>

        <template v-for="offset in [-2, -1, 0, 1, 2]" :key="'offset-' + offset">
          <a-button
            v-if="store.questions[store.currentIndex + offset]"
            size="small"
            class="nav-dot"
            :type="offset === 0 ? 'primary' : 'default'"
            :style="offset !== 0 ? navDotStyle(store.currentIndex + offset) : {}"
            @click="store.goTo(store.currentIndex + offset)"
          >
            {{ store.currentIndex + offset + 1 }}
          </a-button>
        </template>

        <span
          v-if="store.currentIndex < store.totalCount - 4"
          class="nav-ellipsis"
        >…</span>
        <a-button
          v-if="store.currentIndex < store.totalCount - 3"
          size="small"
          class="nav-dot"
          :style="navDotStyle(store.totalCount - 1)"
          @click="store.goTo(store.totalCount - 1)"
        >
          {{ store.totalCount }}
        </a-button>
      </div>

      <!-- 跳转输入 -->
      <div style="text-align: center; margin-top: 8px">
        <a-space>
          <span style="font-size: 12px; color: #999">跳转到</span>
          <a-input-number
            v-model:value="jumpInput"
            :min="1"
            :max="store.totalCount"
            style="width: 70px"
            size="small"
            @pressEnter="handleJump"
          />
          <a-button size="small" @click="handleJump">题</a-button>
        </a-space>
      </div>
    </div>
  </div>
</template>

<style scoped>
.practice-page {
  max-width: 900px;
  margin: 0 auto;
}

.practice-header {
  display: flex;
  align-items: center;
}

.question-stem {
  font-size: 16px;
  line-height: 1.8;
  margin-bottom: 16px;
  padding: 16px;
  background: #fafafa;
  border-radius: 8px;
  border-left: 4px solid #1890ff;
}

.q-number {
  font-weight: bold;
  color: #1890ff;
  margin-right: 8px;
}

.question-stats {
  font-size: 12px;
  color: #999;
  margin-bottom: 16px;
  padding: 8px 12px;
  background: #f5f5f5;
  border-radius: 4px;
}

.options-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.option-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border: 1px solid #d9d9d9;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.option-item:hover {
  border-color: #1890ff;
  background: #e6f7ff;
}

.option-letter {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #f0f0f0;
  font-weight: bold;
  margin-right: 12px;
  flex-shrink: 0;
}

.option-text {
  flex: 1;
}

.option-icon {
  margin-left: 8px;
  font-size: 18px;
}

.option-correct {
  border-color: #52c41a !important;
  background: #f6ffed !important;
}

.option-correct .option-letter {
  background: #52c41a;
  color: #fff;
}

.option-correct .option-icon {
  color: #52c41a;
}

.option-wrong {
  border-color: #f5222d !important;
  background: #fff2f0 !important;
}

.option-wrong .option-letter {
  background: #f5222d;
  color: #fff;
}

.option-wrong .option-icon {
  color: #f5222d;
}

.option-selected {
  border-color: #1890ff !important;
  background: #e6f7ff !important;
}

.option-selected .option-letter {
  background: #1890ff;
  color: #fff;
}

.practice-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #f0f0f0;
}

.question-nav-strip {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 4px;
  margin-top: 10px;
}

.nav-dot {
  min-width: 28px;
  height: 28px;
  padding: 0 4px;
  font-size: 12px;
  border-radius: 4px;
}

.nav-ellipsis {
  color: #999;
  font-size: 14px;
  padding: 0 2px;
  user-select: none;
}
</style>
