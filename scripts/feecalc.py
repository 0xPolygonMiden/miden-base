from dataclasses import dataclass
import math

COMPUTATION_UNIT_PER_CYCLE_BUCKET = 500
COMPUTATION_UNIT_PER_CREATED_NOTE = 50
COMPUTATION_UNIT_PER_CONSUMED_NOTE = 30
COMPUTATION_UNIT_PER_KILO_BYTE = 15


@dataclass
class ComputationSummary:
    cycle_units: int = 0
    notes_consumed_units: int = 0
    notes_created_units: int = 0
    # Total data units comprised of public note and public account delta size.
    data_units: int = 0

    def total_computation_units(self) -> int:
        return (
            self.cycle_units
            + self.notes_consumed_units
            + self.notes_created_units
            + self.data_units
        )


@dataclass
class TransactionMetrics:
    # Number in 2^10..2^29
    num_cycles: int = 0
    # Number in 0..=1024
    num_notes_consumed: int = 0
    # Number in 0..=1024
    num_notes_created: int = 0
    # Note
    #   NoteHeader (serialized as only NoteMetadata) = 32
    #   NoteDetails
    #     NoteAssets = 0..(255 * 32 = 8160)
    #     NoteRecipient
    #       NoteScript = ~1000..~10_000 (?)
    #       NoteInputs = 0..(1 + 255 * 8 = 2041)
    #       SerialNumber = 32
    # Number in ~1000..~20_000
    created_public_notes_byte_size: int = 0
    # Number in 0..2^15
    public_account_delta_byte_size: int = 0

    # Get the cycle bucket of the transaction, which is in 10..29
    def cycle_bucket(self) -> int:
        return math.ceil(math.log2(self.num_cycles))

    # Get the number of kilo bytes of public data in the transaction, rounded up.
    def kilo_bytes(self) -> int:
        return math.ceil(
            (self.created_public_notes_byte_size + self.public_account_delta_byte_size)
            / 1000
        )

    def calculate_computation_units(self) -> ComputationSummary:
        cycle_bucket = self.cycle_bucket()
        kilo_bytes = self.kilo_bytes()

        return ComputationSummary(
            cycle_bucket * COMPUTATION_UNIT_PER_CYCLE_BUCKET,
            self.num_notes_consumed * COMPUTATION_UNIT_PER_CONSUMED_NOTE,
            self.num_notes_created * COMPUTATION_UNIT_PER_CREATED_NOTE,
            kilo_bytes * COMPUTATION_UNIT_PER_KILO_BYTE,
        )


def print_markdown_table(data, headers):
    """Prints a table in GitHub Flavored Markdown format."""
    # Determine column widths
    col_widths = [
        max(len(str(row[i])) for row in [headers] + data) for i in range(len(headers))
    ]

    # Format header row
    header_row = " | ".join(
        f"{str(headers[i]).ljust(col_widths[i])}" for i in range(len(headers))
    )

    # Format separator row
    separator_row = " | ".join("-" * col_widths[i] for i in range(len(headers)))

    # Format data rows
    data_rows = "\n".join(
        " | ".join(f"{str(row[i]).ljust(col_widths[i])}" for i in range(len(headers)))
        for row in data
    )

    # Print the markdown table
    print(
        f"| {header_row} |\n| {separator_row} |\n{''.join(f'| {row} |\n' for row in data_rows.splitlines())}"
    )


if __name__ == "__main__":
    TX_CYCLES_SMALL = 2**16
    TX_CYCLES_LARGE = 2**20
    NUM_NOTES_SMALL = 5
    NUM_NOTES_LARGE = 250
    # The approximate byte size of a P2ID note with one asset.
    P2ID_SIZE = 1_300
    NOTE_SIZE_LARGE = 8_000

    headers = [
        "Cycle Bucket",
        "Notes Consumed",
        "Notes Created",
        "Public Note/Account Data",
        "Computation Units",
    ]
    table = []

    for cycles in [TX_CYCLES_SMALL, TX_CYCLES_LARGE]:
        for num_notes_consumed in [NUM_NOTES_SMALL, NUM_NOTES_LARGE]:
            for num_notes_created in [NUM_NOTES_SMALL, NUM_NOTES_LARGE]:
                for public_note_size in [0, P2ID_SIZE, NOTE_SIZE_LARGE]:
                    total_public_note_size = num_notes_created * public_note_size
                    tx = TransactionMetrics(
                        cycles,
                        num_notes_consumed,
                        num_notes_created,
                        total_public_note_size,
                        0,
                    )
                    summary = tx.calculate_computation_units()

                    table.append(
                        [
                            f"{tx.cycle_bucket()} ({summary.cycle_units:_})",
                            f"{tx.num_notes_consumed} ({summary.notes_consumed_units:_})",
                            f"{tx.num_notes_created} ({summary.notes_created_units:_})",
                            f"{tx.created_public_notes_byte_size + tx.public_account_delta_byte_size:_} ({summary.data_units:_})",
                            f"{summary.total_computation_units():_}",
                        ]
                    )

    print_markdown_table(table, headers)
