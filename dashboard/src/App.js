import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Container } from '@mui/material';
import Header from './components/Header';
import Dashboard from './components/Dashboard';
import AgentList from './components/AgentList';
import AgentDetail from './components/AgentDetail';

function App() {
  return (
    <Router>
      <Header />
      <Container maxWidth="xl" sx={{ mt: 4, mb: 4 }}>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/agents" element={<AgentList />} />
          <Route path="/agents/:id" element={<AgentDetail />} />
        </Routes>
      </Container>
    </Router>
  );
}

export default App;
