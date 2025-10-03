import { createSlice, PayloadAction } from '@reduxjs/toolkit';

interface State_24 {
    data: any[];
    loading: boolean;
    error: string | null;
}

const slice_24 = createSlice({
    name: 'feature_24',
    initialState: { data: [], loading: false, error: null },
    reducers: {
        setData: (state, action: PayloadAction<any[]>) => {
            state.data = action.payload;
        },
        setLoading: (state, action: PayloadAction<boolean>) => {
            state.loading = action.payload;
        },
    },
});

export const { setData, setLoading } = slice_24.actions;
export default slice_24.reducer;