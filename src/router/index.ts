import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
    history: createWebHashHistory(),
    routes: [
        { path: '/', component: () => import('../views/MainPanel.vue') },
        { path: '/widget', component: () => import('../views/WidgetIsland.vue') }
    ]
})

export default router