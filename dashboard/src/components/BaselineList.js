import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Table, TableBody, TableCell, TableContainer, TableHead, TableRow,
  Paper, Typography, Box, CircularProgress, Alert, TextField
} from '@mui/material';
import { agentsAPI } from '../services/api';

function BaselineList() {
  const [search, setSearch] = useState('');
  const { data: baselines, isLoading, error } = useQuery({
    queryKey: ['baselines'],
    queryFn: agentsAPI.getBaselines,
  });

  if (isLoading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="60vh">
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return <Alert severity="error">Error loading baselines: {error.message}</Alert>;
  }

  const filtered = (baselines || []).filter(b =>
    b.imageId?.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <Box>
      <Typography variant="h4" gutterBottom>Baselines</Typography>
      <TextField
        fullWidth label="Search by Image ID" variant="outlined"
        value={search} onChange={(e) => setSearch(e.target.value)}
        sx={{ mb: 3 }}
      />
      {filtered.length === 0 ? (
        <Typography color="textSecondary">No baselines found.</Typography>
      ) : (
        <TableContainer component={Paper}>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Image ID</TableCell>
                <TableCell>Timestamp</TableCell>
                <TableCell>File Count</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filtered.map((baseline) => (
                <TableRow key={baseline.imageId}>
                  <TableCell>{baseline.imageId}</TableCell>
                  <TableCell>{new Date(baseline.timestamp).toLocaleString()}</TableCell>
                  <TableCell>{baseline.fileCount}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      )}
    </Box>
  );
}

export default BaselineList;
