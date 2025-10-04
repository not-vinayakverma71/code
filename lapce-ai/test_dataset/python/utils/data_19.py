import pandas as pd
import numpy as np

def process_dataframe_19(df: pd.DataFrame) -> pd.DataFrame:
    # Clean data
    df = df.dropna()
    df = df[df['value'] > 0]
    
    # Transform
    df['normalized'] = (df['value'] - df['value'].mean()) / df['value'].std()
    
    return df

def calculate_statistics_19(data: list) -> dict:
    arr = np.array(data)
    return {
        'mean': np.mean(arr),
        'std': np.std(arr),
        'median': np.median(arr),
    }