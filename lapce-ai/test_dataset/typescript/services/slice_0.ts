import { createSlice, PayloadAction } from '@reduxjs/toolkit';

interface State_0 {
    data: any[];
    loading: boolean;
    error: string | null;
}

const slice_0 = createSlice({
    name: 'feature_0',
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

export const { setData, setLoading } = slice_0.actions;
export default slice_0.reducer;