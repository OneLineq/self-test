// ============================================================
// 刷题助手 — 路由配置
// ============================================================
import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'Dashboard',
      component: () => import('../views/Dashboard.vue'),
    },
    // 四种练习模式题库选择页
    {
      path: '/sequential',
      name: 'Sequential',
      component: () => import('../views/PracticeSelector.vue'),
    },
    {
      path: '/random',
      name: 'Random',
      component: () => import('../views/PracticeSelector.vue'),
    },
    {
      path: '/wrong',
      name: 'Wrong',
      component: () => import('../views/PracticeSelector.vue'),
    },
    {
      path: '/exam',
      name: 'ExamSelector',
      component: () => import('../views/PracticeSelector.vue'),
    },
    // 实际刷题页面（选择题库后进入）
    {
      path: '/practice/:bankId',
      name: 'Practice',
      component: () => import('../views/Practice.vue'),
    },
    {
      path: '/exam/:bankId',
      name: 'MockExam',
      component: () => import('../views/MockExam.vue'),
    },
    {
      path: '/questions/:bankId',
      name: 'QuestionManage',
      component: () => import('../views/QuestionManage.vue'),
    },
    {
      path: '/wrong-manage/:bankId',
      name: 'WrongManage',
      component: () => import('../views/WrongManage.vue'),
    },
    {
      path: '/banks',
      name: 'BankManage',
      component: () => import('../views/BankManage.vue'),
    },
    {
      path: '/data',
      name: 'DataManage',
      component: () => import('../views/DataManage.vue'),
    },
    {
      path: '/memory',
      name: 'PracticeMemory',
      component: () => import('../views/Memory.vue'),
    },
    {
      path: '/guide',
      name: 'UserGuide',
      component: () => import('../views/UserGuide.vue'),
    },
  ],
})

export default router
