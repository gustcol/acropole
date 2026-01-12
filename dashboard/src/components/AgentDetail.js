import React from 'react';
import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import {
  Grid,
  Card,
  CardContent,
  Typography,
  Box,
  CircularProgress,
  Alert,
  Chip,
  List,
  ListItem,
  ListItemText,
  Divider,
} from '@mui/material';
import {
  CheckCircle as CheckCircleIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  Computer as ComputerIcon,
} from '@mui/icons-material';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { agentsAPI } from '../services/api';

function AgentDetail() {
  const { id } = useParams();

  const { data: agent, isLoading: agentLoading, error: agentError } = useQuery({
    queryKey: ['agent', id],
    queryFn: () => agentsAPI.getAgent(id),
  });

  const { data: heartbeats, isLoading: heartbeatsLoading } = useQuery({
    queryKey: ['agentHeartbeats', id],
    queryFn: () => agentsAPI.getAgentHeartbeats(id),
  });

  const { data: alerts, isLoading: alertsLoading } = useQuery({
    queryKey: ['agentAlerts', id],
    queryFn: () => agentsAPI.getAgentAlerts(id),
  });

  if (agentLoading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="60vh">
        <CircularProgress />
      </Box>
    );
  }

  if (agentError) {
    return (
      <Alert severity="error">
        Error loading agent details: {agentError.message}
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

  const getStatusIcon = (status) => {
    switch (status?.toLowerCase()) {
      case 'healthy':
        return <CheckCircleIcon color="success" />;
      case 'warning':
        return <WarningIcon color="warning" />;
      case 'critical':
        return <ErrorIcon color="error" />;
      default:
        return <ComputerIcon />;
    }
  };

  // Prepare heartbeat data for chart
  const heartbeatChartData = heartbeats?.slice(-20).map(hb => ({
    time: new Date(hb.timestamp).toLocaleTimeString(),
    status: hb.status === 'healthy' ? 1 : hb.status === 'warning' ? 0.5 : 0,
  })) || [];

  return (
    <Box>
      <Typography variant="h4" gutterBottom>
        Agent Details
      </Typography>

      <Grid container spacing={3}>
        {/* Agent Info Card */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Box display="flex" alignItems="center" mb={2}>
                {getStatusIcon(agent.status)}
                <Typography variant="h5" ml={1}>
                  {agent.hostname}
                </Typography>
              </Box>

              <Grid container spacing={2}>
                <Grid item xs={6}>
                  <Typography color="textSecondary" variant="body2">
                    IP Address
                  </Typography>
                  <Typography variant="body1">
                    {agent.ipAddress}
                  </Typography>
                </Grid>

                <Grid item xs={6}>
                  <Typography color="textSecondary" variant="body2">
                    Status
                  </Typography>
                  <Chip
                    label={agent.status}
                    color={getStatusColor(agent.status)}
                    size="small"
                  />
                </Grid>

                <Grid item xs={6}>
                  <Typography color="textSecondary" variant="body2">
                    Last Heartbeat
                  </Typography>
                  <Typography variant="body1">
                    {agent.lastHeartbeat ? new Date(agent.lastHeartbeat).toLocaleString() : 'Never'}
                  </Typography>
                </Grid>

                <Grid item xs={6}>
                  <Typography color="textSecondary" variant="body2">
                    Active Alerts
                  </Typography>
                  <Typography variant="body1">
                    {agent.alertCount || 0}
                  </Typography>
                </Grid>
              </Grid>
            </CardContent>
          </Card>
        </Grid>

        {/* Recent Heartbeats Chart */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Recent Heartbeats
              </Typography>
              {heartbeatsLoading ? (
                <Box display="flex" justifyContent="center" py={4}>
                  <CircularProgress />
                </Box>
              ) : (
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={heartbeatChartData}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="time" />
                    <YAxis domain={[0, 1]} ticks={[0, 0.5, 1]} />
                    <Tooltip
                      formatter={(value) => [value === 1 ? 'Healthy' : value === 0.5 ? 'Warning' : 'Critical', 'Status']}
                    />
                    <Line
                      type="monotone"
                      dataKey="status"
                      stroke="#1976d2"
                      strokeWidth={2}
                      dot={{ r: 3 }}
                    />
                  </LineChart>
                </ResponsiveContainer>
              )}
            </CardContent>
          </Card>
        </Grid>

        {/* Recent Alerts */}
        <Grid item xs={12}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Recent Alerts
              </Typography>
              {alertsLoading ? (
                <Box display="flex" justifyContent="center" py={4}>
                  <CircularProgress />
                </Box>
              ) : alerts && alerts.length > 0 ? (
                <List>
                  {alerts.slice(0, 10).map((alert, index) => (
                    <React.Fragment key={alert.id}>
                      <ListItem>
                        <ListItemText
                          primary={alert.message}
                          secondary={
                            <Box>
                              <Typography variant="body2" color="textSecondary">
                                {new Date(alert.timestamp).toLocaleString()}
                              </Typography>
                              <Chip
                                label={alert.severity}
                                color={alert.severity === 'critical' ? 'error' : alert.severity === 'warning' ? 'warning' : 'info'}
                                size="small"
                                sx={{ mt: 1 }}
                              />
                            </Box>
                          }
                        />
                      </ListItem>
                      {index < alerts.slice(0, 10).length - 1 && <Divider />}
                    </React.Fragment>
                  ))}
                </List>
              ) : (
                <Typography color="textSecondary">
                  No recent alerts for this agent.
                </Typography>
              )}
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Box>
  );
}

export default AgentDetail;
