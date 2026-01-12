import axios from 'axios';

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:8080';

const api = axios.create({
  baseURL: API_BASE_URL,
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Request interceptor for logging
api.interceptors.request.use(
  (config) => {
    console.log(`API Request: ${config.method?.toUpperCase()} ${config.url}`);
    return config;
  },
  (error) => {
    console.error('API Request Error:', error);
    return Promise.reject(error);
  }
);

// Response interceptor for error handling
api.interceptors.response.use(
  (response) => {
    console.log(`API Response: ${response.status} ${response.config.url}`);
    return response;
  },
  (error) => {
    console.error('API Response Error:', error.response?.data || error.message);
    return Promise.reject(error);
  }
);

export const agentsAPI = {
  // Get all agents
  getAgents: () => api.get('/agents').then(res => res.data),

  // Get specific agent
  getAgent: (id) => api.get(`/agents/${id}`).then(res => res.data),

  // Get agent heartbeats
  getAgentHeartbeats: (id) => api.get(`/agents/${id}/heartbeats`).then(res => res.data),

  // Get all alerts
  getAlerts: () => api.get('/alerts').then(res => res.data),

  // Get alerts for specific agent
  getAgentAlerts: (id) => api.get(`/agents/${id}/alerts`).then(res => res.data),

  // Get dashboard summary
  getDashboardSummary: () => api.get('/dashboard/summary').then(res => res.data),
};

export default api;
