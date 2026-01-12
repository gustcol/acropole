import React from 'react';
import { AppBar, Toolbar, Typography, Button, Box } from '@mui/material';
import { Dashboard as DashboardIcon, Storage as StorageIcon } from '@mui/icons-material';
import { useNavigate, useLocation } from 'react-router-dom';

function Header() {
  const navigate = useNavigate();
  const location = useLocation();

  const isActive = (path) => location.pathname === path;

  return (
    <AppBar position="static">
      <Toolbar>
        <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
          Golden Image Integrity System
        </Typography>

        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            color="inherit"
            startIcon={<DashboardIcon />}
            onClick={() => navigate('/')}
            variant={isActive('/') ? 'outlined' : 'text'}
          >
            Dashboard
          </Button>

          <Button
            color="inherit"
            startIcon={<StorageIcon />}
            onClick={() => navigate('/agents')}
            variant={isActive('/agents') ? 'outlined' : 'text'}
          >
            Agents
          </Button>
        </Box>
      </Toolbar>
    </AppBar>
  );
}

export default Header;
