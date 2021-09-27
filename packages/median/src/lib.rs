use num_traits::{cast, PrimInt};

#[derive(Debug, Clone, PartialEq)]
pub struct EmptyArrayError;

pub fn calculate_median<T>(entries: &mut [T]) -> Result<T, EmptyArrayError>
where
    T: PrimInt,
{
    if entries.is_empty() {
        return Err(EmptyArrayError);
    }
    entries.sort_unstable();

    let mid = entries.len() / 2;
    let median = match entries.len() % 2 {
        0 => (entries[mid - 1] + entries[mid]) / cast(2).unwrap(),
        _ => entries[mid],
    };

    Ok(median)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_median() {
        let mut entries = (1..5_u128).collect::<Vec<u128>>();
        let median_even = calculate_median(&mut entries).unwrap();
        assert_eq!(median_even, 2_u128);

        let mut entries = (1..=5_u128).collect::<Vec<u128>>();
        let median_odd = calculate_median(&mut entries).unwrap();
        assert_eq!(median_odd, 3_u128);
    }

    #[test]
    fn test_calculate_median_empty_arr() {
        let median_err = calculate_median::<u32>(&mut vec![]).unwrap_err();
        assert_eq!(median_err, EmptyArrayError);
    }
}
