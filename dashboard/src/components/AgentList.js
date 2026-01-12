import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import {
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Chip,
  IconButton,
  TextField,
  Box,
  CircularProgress,
  Alert,
  Typography,
} from '@mui/material';
import { Visibility as VisibilityIcon } from '@mui/icons-material';
import { agentsAPI } from '../services/api';

function AgentList() {
  const navigate = useNavigate();
  const [searchTerm, setSearchTerm] = useState('');

  const { data: agents, isLoading, error } = useQuery({
    queryKey: ['agents'],
    queryFn: agentsAPI.getAgents,
  });

  if (isLoading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="60vh">
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return (
      <Alert severity="error">
        Error loading agents: {error.message}
      </Alert>
    );
  }

  const getStatusColor = (status) => {
    switch (status?.toLowerCase()) {
      case 'healthy':
        return 'success';
      case 'warning':
        return 'warning';
      case 'critical':
        return 'error';
      default:
        return 'default';
    }
  };

  const filteredAgents = agents?.filter(agent =>
    agent.hostname?.toLowerCase().includes(searchTerm.toLowerCase()) ||
    agent.ipAddress?.includes(searchTerm) ||
    agent.status?.toLowerCase().includes(searchTerm.toLowerCase())
  ) || [];

  return (
    <Box>
      <Typography variant="h4" gutterBottom>
        Agents
      </Typography>

      <Box mb={3}>
        <TextField
          fullWidth
          label="Search agents..."
          variant="outlined"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
        />
      </Box>

      <TableContainer component={Paper}>
        <Table>
          <TableHead>
            <TableRow>
              <TableCell>Hostname</TableCell>
              <TableCell>IP Address</TableCell>
              <TableCell>Status</TableCell>
              <TableCell>Last Heartbeat</TableCell>
              <TableCell>Alerts</TableCell>
              <TableCell>Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {filteredAgents.map((agent) => (
              <TableRow key={agent.id}>
                <TableCell>{agent.hostname}</TableCell>
                <TableCell>{agent.ipAddress}</TableCell>
                <TableCell>
                  <Chip
                    label={agent.status}
                    color={getStatusColor(agent.status)}
                    size="small"
                  />
                </TableCell>
                <TableCell>
                  {agent.lastHeartbeat ? new Date(agent.lastHeartbeat).toLocaleString() : 'Never'}
                </TableCell>
                <TableCell>
                  <Chip
                    label={agent.alertCount || 0}
                    color={agent.alertCount > 0 ? 'error' : 'default'}
                    size="small"
                  />
                </TableCell>
                <TableCell>
                  <IconButton
                    onClick={() => navigate(`/agents/${agent.id}`)}
                    color="primary"
                    size="small"
                  >
                    <VisibilityIcon />
                  </IconButton>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>

      {filteredAgents.length === 0 && (
        <Box textAlign="center" py={4}>
          <Typography color="textSecondary">
            No agents found matching your search.
          </Typography>
        </Box>
      )}
    </Box>
  );
}

export default AgentList;
