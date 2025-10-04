import { createSlice, PayloadAction } from '@reduxjs/toolkit';

interface State_48 {
    data: any[];
    loading: boolean;
    error: string | null;
}

const slice_48 = createSlice({
    name: 'feature_48',
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

export const { setData, setLoading } = slice_48.actions;
export default slice_48.reducer;