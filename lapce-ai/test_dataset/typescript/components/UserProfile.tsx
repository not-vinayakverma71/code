import React, { useState, useEffect } from 'react';
import { useQuery, useMutation } from '@apollo/client';

interface User {
    id: string;
    name: string;
    email: string;
}

export const UserProfile: React.FC<{ userId: string }> = ({ userId }) => {
    const [isEditing, setIsEditing] = useState(false);
    const { data, loading, error } = useQuery(GET_USER_PROFILE, {
        variables: { userId },
    });
    
    const [updateProfile] = useMutation(UPDATE_USER_PROFILE);
    
    if (loading) return <div>Loading...</div>;
    if (error) return <div>Error: {error.message}</div>;
    
    return (
        <div className="user-profile">
            <h2>{data.user.name}</h2>
            <p>{data.user.email}</p>
        </div>
    );
};